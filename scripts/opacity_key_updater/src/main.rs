use std::{collections::BTreeSet, env, fs, path::PathBuf, time::Duration};

use opacity_verifier::msg::ExecuteMsg;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdaterError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error("address hex must be 40 chars")] 
    BadHexLen,
    #[error("invalid hex address: {0}")]
    BadHex(String),
    #[error("CONFIG: CONTRACT_ADDRESS must be set to submit updates")] 
    MissingContract,
    #[error("unexpected response shape: {0}")]
    UnexpectedShape(String),
}

#[derive(Clone, Debug)]
struct Config {
    keys_url: String,
    keys_field: Option<String>,
    poll_secs: u64,
    state_path: PathBuf,
    dry_run: bool,
    // chain submission configuration (used when DRY_RUN=false)
    contract_addr: Option<String>,
    rpc_endpoint: Option<String>,
    grpc_endpoint: Option<String>,
    chain_id: Option<String>,
    gas_denom: Option<String>,
    gas_price: Option<String>,
    bech32_prefix: Option<String>,
    admin_mnemonic: Option<String>,
}

impl Config {
    fn from_env() -> Self {
        let keys_url = env::var("KEYS_URL").unwrap_or_else(|_| "https://verifier.opacity.network/api/public-keys".to_string());
        let keys_field = env::var("KEYS_FIELD").ok();
        let poll_secs = env::var("POLL_INTERVAL_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
        let state_path = env::var("STATE_PATH").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(".opacity_keys_state.json"));
        let dry_run = env::var("DRY_RUN").ok().map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(true);

        let contract_addr = env::var("CONTRACT_ADDRESS").ok();
        let rpc_endpoint = env::var("RPC_ENDPOINT").ok();
        let grpc_endpoint = env::var("GRPC_ENDPOINT").ok();
        let chain_id = env::var("CHAIN_ID").ok();
        let gas_denom = env::var("GAS_DENOM").ok();
        let gas_price = env::var("GAS_PRICE").ok(); // e.g., "0.025uxion"
        let bech32_prefix = env::var("BECH32_PREFIX").ok();
        let admin_mnemonic = env::var("ADMIN_MNEMONIC").ok();

        Self { keys_url, keys_field, poll_secs, state_path, dry_run, contract_addr, rpc_endpoint, grpc_endpoint, chain_id, gas_denom, gas_price, bech32_prefix, admin_mnemonic }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    keys: BTreeSet<String>,
    hash: String,
}

async fn fetch_keys(url: &str, keys_field: Option<&str>) -> Result<Vec<String>, UpdaterError> {
    let client = reqwest::Client::builder().build()?;
    let res = client.get(url).send().await?.error_for_status()?;
    // Accept either a top-level array of strings, or an object with the configured field name
    let v: serde_json::Value = res.json().await?;
    if let Some(arr) = v.as_array() {
        let keys: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
        return Ok(keys);
    }
    if let Some(obj) = v.as_object() {
        if let Some(name) = keys_field {
            if let Some(arr) = obj.get(name).and_then(|k| k.as_array()) {
                let keys: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect();
                return Ok(keys);
            }
            return Err(UpdaterError::UnexpectedShape(format!(
                "object did not contain configured KEYS_FIELD='{}' as an array; available fields = {:?}",
                name,
                obj.keys().cloned().collect::<Vec<_>>()
            )));
        }
        return Err(UpdaterError::UnexpectedShape(format!(
            "object response without configured KEYS_FIELD; set KEYS_FIELD to the array field name. Available fields = {:?}",
            obj.keys().cloned().collect::<Vec<_>>()
        )));
    }
    Err(UpdaterError::UnexpectedShape(format!("top-level type: {}", v.to_string())))
}

fn normalize_addr_hex(s: &str) -> Result<String, UpdaterError> {
    let s = s.trim().trim_start_matches("0x").to_ascii_lowercase();
    if s.len() != 40 { return Err(UpdaterError::BadHexLen); }
    hex::decode(&s).map_err(|_| UpdaterError::BadHex(s.clone()))?;
    Ok(s)
}

fn hash_keys(set: &BTreeSet<String>) -> String {
    let mut hasher = Sha256::new();
    for k in set.iter() {
        hasher.update(k.as_bytes());
        hasher.update(&[0u8]);
    }
    let out = hasher.finalize();
    hex::encode(out)
}

fn load_state(path: &PathBuf) -> Result<Option<State>, UpdaterError> {
    if !path.exists() { return Ok(None); }
    let data = fs::read(path)?;
    let st: State = serde_json::from_slice(&data)?;
    Ok(Some(st))
}

fn save_state(path: &PathBuf, state: &State) -> Result<(), UpdaterError> {
    let tmp = serde_json::to_vec_pretty(state)?;
    fs::write(path, tmp)?;
    Ok(())
}

async fn submit_update(cfg: &Config, keys: Vec<String>) -> Result<(), UpdaterError> {
    use cw_orch::prelude::*;
    use std::collections::BTreeSet;

    let contract_addr = cfg.contract_addr.clone().ok_or(UpdaterError::MissingContract)?;
    let chain_id = cfg.chain_id.clone().expect("CHAIN_ID required when submitting");
    let gas_denom = cfg.gas_denom.clone().expect("GAS_DENOM required when submitting");
    let rpc = cfg.rpc_endpoint.clone().expect("RPC_ENDPOINT required when submitting");
    let grpc = cfg.grpc_endpoint.clone().unwrap_or_else(|| rpc.replace("http", "https"));
    let prefix = cfg.bech32_prefix.clone().expect("BECH32_PREFIX required when submitting");
    let mnemonic = cfg.admin_mnemonic.clone().expect("ADMIN_MNEMONIC required when submitting");
    let gas_price = cfg.gas_price.clone().unwrap_or_else(|| format!("0.025{}", gas_denom));

    // Build chain info
    use cw_orch::daemon::{Daemon, DaemonBuilder};
    use cw_orch::environment::{ChainInfoOwned, NetworkInfoOwned, ChainKind};

    let mut chain = ChainInfoOwned::default();
    chain.chain_id = chain_id.clone();
    chain.gas_denom = gas_denom.clone();
    chain.gas_price = 0.0; // overridden by DaemonBuilder::gas_price below
    chain.grpc_urls = vec![grpc.clone()];
    chain.lcd_url = None;
    chain.fcd_url = None;
    chain.network_info = NetworkInfoOwned { chain_name: chain_id.clone(), pub_address_prefix: prefix.clone(), coin_type: 118 };
    chain.kind = ChainKind::Unspecified;

    // Parse numeric gas price (e.g., "0.025uxion" -> 0.025)
    let gp_num: f64 = gas_price
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .parse()
        .unwrap_or(0.025);

    let mut builder = DaemonBuilder::new(chain);
    builder.grpc_url(grpc.clone());
    builder.mnemonic(&mnemonic);
    builder.gas(Some(&gas_denom), Some(gp_num));

    let daemon = builder
        .build()
        .expect("failed to build daemon");

    #[cw_orch::interface(opacity_verifier::msg::InstantiateMsg, opacity_verifier::msg::ExecuteMsg, opacity_verifier::msg::QueryMsg, cosmwasm_std::Empty)]
    struct OpacityVerifier;

    let contract: OpacityVerifier<Daemon> = OpacityVerifier::new(&contract_addr, daemon);

    // Before submitting, pull current on-chain keys and compare
    let onchain_keys: Vec<String> = contract
        .query(&opacity_verifier::msg::QueryMsg::VerificationKeys {})
        .expect("failed to query VerificationKeys");

    let mut onchain_set: BTreeSet<String> = BTreeSet::new();
    for k in onchain_keys {
        // Normalize just in case, though contract already stores normalized
        let n = normalize_addr_hex(&k).unwrap_or_else(|_| k);
        onchain_set.insert(n);
    }

    let mut new_set: BTreeSet<String> = BTreeSet::new();
    for k in keys.iter() {
        let n = normalize_addr_hex(k).unwrap_or_else(|_| k.clone());
        new_set.insert(n);
    }

    if onchain_set == new_set {
        println!("On-chain allowlist already up to date ({} keys). Skipping submit.", new_set.len());
        return Ok(());
    }

    // Execute update
    contract
        .execute(&ExecuteMsg::UpdateAllowList { keys }, &[])
        .expect("failed to submit UpdateAllowList");

    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), UpdaterError> {
    let cfg = Config::from_env();
    println!("opacity_key_updater started. Polling {} every {}s. Dry-run: {}. Keys field: {}", cfg.keys_url, cfg.poll_secs, cfg.dry_run, cfg.keys_field.as_deref().unwrap_or("<top-level array>"));

    loop {
        if let Err(e) = run_once(&cfg).await {
            eprintln!("[WARN] run_once failed: {e}");
        }
        tokio::time::sleep(Duration::from_secs(cfg.poll_secs)).await;
    }
}

async fn run_once(cfg: &Config) -> Result<(), UpdaterError> {
    let raw = fetch_keys(&cfg.keys_url, cfg.keys_field.as_deref()).await?;
    let mut set: BTreeSet<String> = BTreeSet::new();
    for k in raw {
        let n = normalize_addr_hex(&k)?;
        set.insert(n);
    }

    let new_hash = hash_keys(&set);

    let prev = load_state(&cfg.state_path)?;
    if let Some(prev) = prev {
        if prev.hash == new_hash {
            println!("No change detected ({} keys).", set.len());
            return Ok(());
        } else {
            let removed: Vec<_> = prev.keys.difference(&set).cloned().collect();
            let added: Vec<_> = set.difference(&prev.keys).cloned().collect();
            println!("Change detected: +{} / -{} (total now {}).", added.len(), removed.len(), set.len());
            if cfg.dry_run {
                println!("DRY-RUN: would submit UpdateAllowList with {} keys.", set.len());
                // Still write state so subsequent run only submits once when toggling off dry-run
                save_state(&cfg.state_path, &State { keys: set.clone(), hash: new_hash })?;
            } else {
                // Submit update then persist
                submit_update(cfg, set.iter().cloned().collect()).await?;
                save_state(&cfg.state_path, &State { keys: set.clone(), hash: new_hash })?;
            }
        }
    } else {
        println!("No previous state; treating as first run with {} keys.", set.len());
        if cfg.dry_run {
            println!("DRY-RUN: would submit initial UpdateAllowList with {} keys.", set.len());
            save_state(&cfg.state_path, &State { keys: set.clone(), hash: new_hash })?;
        } else {
            submit_update(cfg, set.iter().cloned().collect()).await?;
            save_state(&cfg.state_path, &State { keys: set.clone(), hash: new_hash })?;
        }
    }

    Ok(())
}

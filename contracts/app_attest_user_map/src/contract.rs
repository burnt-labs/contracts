use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::InstantiateMsg;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{APP_ID, user_map, BacReading, BacResponse};
use cosmwasm_std::{entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult};
use cw_storage_plus::PrefixBound;
use crate::error::ContractError::InvalidAppId;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    APP_ID.save(deps.storage, &msg.app_id)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

/// Challenge JSON shape from the app:
/// {"bac": 0.042, "user": "xion1...", "timestamp": 1709901234567}
#[derive(serde::Deserialize)]
struct ChallengePayload {
    bac: f64,
    timestamp: u64, // milliseconds since epoch
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    let map = user_map();
    match msg {
        ExecuteMsg::Sweep {} => {
            let now = env.block.time.seconds();
            let expired_addrs: Vec<Addr> = map
                .idx
                .expiry
                .prefix_range(
                    deps.storage,
                    None,
                    Some(PrefixBound::inclusive(now)),
                    Order::Ascending,
                )
                .filter_map(|item| item.ok().map(|(addr, _)| addr))
                .collect();

            let count = expired_addrs.len();
            for addr in &expired_addrs {
                map.remove(deps.storage, addr)?;
            }

            Ok(Response::new()
                .add_attribute("action", "sweep")
                .add_attribute("removed", count.to_string()))
        }
        ExecuteMsg::Update { attestation } => {
            // require app id to match
            if attestation.app_id != APP_ID.load(deps.storage)? {
                return Err(InvalidAppId)
            }

            // verify the attestation
            ios_app_attest::verify_attestation(
                attestation.app_id,
                attestation.key_id,
                attestation.challenge.clone(),
                attestation.cbor_data,
                env.block.time.seconds() as i64,
                attestation.dev_env,
            )?;

            // parse the verified challenge payload
            let payload: ChallengePayload = serde_json::from_slice(&attestation.challenge)?;

            // convert BAC to millis (0.042 → 42)
            let bac_millis = (payload.bac * 1000.0).round() as u32;

            // use the earlier of submitted timestamp and block time
            let submitted_secs = payload.timestamp / 1000;
            let block_secs = env.block.time.seconds();
            let timestamp = submitted_secs.min(block_secs);

            let reading = BacReading {
                bac_millis,
                timestamp,
            };

            // save handles old index removal + new index creation automatically
            map.save(deps.storage, &info.sender, &reading)?;

            Ok(Response::new()
                .add_attribute("action", "update_bac")
                .add_attribute("bac_millis", bac_millis.to_string())
                .add_attribute("timestamp", timestamp.to_string()))
        }
    }
}

fn reading_to_response(reading: &BacReading, now_secs: u64) -> BacResponse {
    BacResponse {
        bac_millis: reading.bac_millis,
        current_bac_millis: reading.current_bac_millis(now_secs),
        timestamp: reading.timestamp,
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let now = env.block.time.seconds();
    let map = user_map();
    match msg {
        QueryMsg::GetValueByUser { address } => {
            let reading = map.load(deps.storage, &address)?;
            to_json_binary(&reading_to_response(&reading, now))
        }
        QueryMsg::GetUsers {} => {
            let mut addrs: Vec<Addr> = Vec::new();
            for addr in map.keys(deps.storage, None, None, Order::Ascending) {
                addrs.push(addr?)
            }
            to_json_binary(&addrs)
        }
        QueryMsg::GetMap {} => {
            let mut response: Vec<(Addr, BacResponse)> = Vec::new();
            for item in map.range(deps.storage, None, None, Order::Ascending) {
                let (key, reading) = item?;
                response.push((key, reading_to_response(&reading, now)))
            }
            to_json_binary(&response)
        }
    }
}

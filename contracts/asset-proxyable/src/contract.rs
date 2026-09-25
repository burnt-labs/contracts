use asset::{contracts::asset_base, traits::DefaultAssetContract};
use cosmwasm_std::{
    Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult, ensure,
    to_json_binary,
};
use cw_utils::nonpayable;
use cw721::{
    Action, DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg,
    state::{CREATOR, Cw721Config},
    traits::Cw721Execute,
};

use crate::{
    CONTRACT_NAME, CONTRACT_VERSION,
    error::{ContractError, ContractResult},
    msg::{
        BaseExecuteMsg, InstantiateMsg, MigrateMsg, ProxyAction, ProxyMsg, ProxyQueryMsg,
        ProxyableExecuteMsg, ProxyableQueryMsg,
    },
    state::{
        MAX_TRUSTED_PROXIES, TRUSTED_PROXIES, clear_trusted_proxies, count_trusted_proxies,
        has_trusted_proxies, list_trusted_proxies,
    },
};

type BaseContract<'a> = DefaultAssetContract<
    'a,
    DefaultOptionalNftExtension,
    DefaultOptionalNftExtensionMsg,
    DefaultOptionalCollectionExtension,
    DefaultOptionalCollectionExtensionMsg,
>;

/// `(cw2 name, cw2 version)` pairs this code accepts as a migration source.
///
/// The current version is listed deliberately, so migrating onto the code a contract
/// already runs is accepted rather than rejected. That is a no-op in practice: the
/// delegated cw721 migration rewrites the cw2 metadata to the same values and its legacy
/// steps do nothing once minter and creator are set. Allowing it keeps a redeploy of the
/// same code id, or a re-run of a migration that was interrupted, from needing a contract
/// change, and it costs nothing because a migration is already gated on the x/wasm admin.
/// Entries are added, never removed, so an older contract can always reach the newest code.
///
/// Both base versions are listed so a collection can move onto this variant from either,
/// and the variant's own version so a variant-to-variant migration keeps its trusted-proxy
/// set (a base source clears it instead; see `migrate`).
pub const MIGRATABLE_FROM: &[(&str, &str)] = &[
    ("asset", "0.1.0"),
    ("asset", "0.2.0"),
    (CONTRACT_NAME, "0.1.0"),
];

/// Same as the base instantiate, but records this crate's cw2 identity so a variant
/// deployment is never mistaken for a base one (and can later pass its own migrate gate).
#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    let contract: BaseContract<'static> = DefaultAssetContract::default();
    Ok(contract.instantiate_with_version(
        deps,
        &env,
        &info,
        msg,
        CONTRACT_NAME,
        CONTRACT_VERSION,
    )?)
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ProxyableExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute { sender, action }) => {
            execute_proxied(deps, env, info, sender, action)
        }
        ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            proxy,
            require_immutable,
        }) => add_trusted_proxy(deps, env, info, proxy, require_immutable),
        ProxyableExecuteMsg::Proxy(ProxyMsg::RemoveTrustedProxy { proxy }) => {
            remove_trusted_proxy(deps, env, info, proxy)
        }
        ProxyableExecuteMsg::Base(base) => execute_base(deps, env, info, base),
    }
}

/// Delegate a base message, guarding the creator-ownership transitions that interact with
/// proxy trust:
/// - renouncing is refused while any proxy is registered, so trust stays revocable;
/// - accepting ownership from a *different* previous creator clears every registered
///   proxy, so an outgoing creator cannot leave (or sneak in during the pending window) an
///   address that keeps collection-wide power after control has changed hands. The new
///   creator re-registers what they trust. A self-accept (the cw-ownable idiom for
///   cancelling a proposal) changes no control and clears nothing.
fn execute_base(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    base: BaseExecuteMsg,
) -> ContractResult<Response> {
    match &base {
        BaseExecuteMsg::UpdateCreatorOwnership(Action::RenounceOwnership)
            if has_trusted_proxies(deps.storage) =>
        {
            Err(ContractError::TrustedProxiesNotEmpty {})
        }
        BaseExecuteMsg::UpdateCreatorOwnership(Action::AcceptOwnership) => {
            let previous = CREATOR.item.load(deps.storage)?.owner;
            let response = asset_base::execute(deps.branch(), env, info, base)?;
            let current = CREATOR.item.load(deps.storage)?.owner;
            let cleared = if previous != current {
                clear_trusted_proxies(deps.storage)?
            } else {
                0
            };
            Ok(response.add_attribute("trusted_proxies_cleared", cleared.to_string()))
        }
        _ => Ok(asset_base::execute(deps, env, info, base)?),
    }
}

/// A creator transfer is live when someone other than the current creator could still
/// accept it. Expired proposals cannot be accepted (cw-ownable rejects them) and a
/// proposal to the current creator is the cancel idiom, so neither blocks anything.
fn creator_transfer_is_live(deps: Deps, env: &Env) -> ContractResult<bool> {
    let ownership = CREATOR.item.load(deps.storage)?;
    let Some(pending) = ownership.pending_owner else {
        return Ok(false);
    };
    if ownership.owner.as_ref() == Some(&pending) {
        return Ok(false);
    }
    let expired = ownership
        .pending_expiry
        .map(|e| e.is_expired(&env.block))
        .unwrap_or(false);
    Ok(!expired)
}

/// Run one of the closed set of `ProxyAction`s as `sender`, on behalf of a trusted proxy.
fn execute_proxied(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: String,
    action: ProxyAction,
) -> ContractResult<Response> {
    nonpayable(&info)?;
    ensure!(
        TRUSTED_PROXIES.has(deps.storage, &info.sender),
        ContractError::Unauthorized {}
    );
    let sender = deps.api.addr_validate(&sender)?;

    let (action_name, mapped) = match action {
        ProxyAction::Burn { token_id } => {
            // Stricter than a direct burn: no operator or approval path through a proxy.
            let nft = Cw721Config::<DefaultOptionalNftExtension>::default()
                .nft_info
                .load(deps.storage, &token_id)?;
            ensure!(
                nft.owner == sender,
                ContractError::NotTokenOwner {
                    token_id: token_id.clone()
                }
            );
            ("burn", BaseExecuteMsg::Burn { token_id })
        }
        ProxyAction::ApproveAll { operator, expires } => (
            "approve_all",
            BaseExecuteMsg::ApproveAll { operator, expires },
        ),
        ProxyAction::RevokeAll { operator } => {
            ("revoke_all", BaseExecuteMsg::RevokeAll { operator })
        }
    };

    // The base contract runs its own authorization (check_can_send, operator storage,
    // burn-while-listed guard) against the effective sender, so a proxied call can never
    // do more than the same user could directly.
    let proxied_info = MessageInfo {
        sender: sender.clone(),
        funds: vec![],
    };
    let response = asset_base::execute(deps, env, proxied_info, mapped)?;
    Ok(response
        .add_attribute("proxied_by", info.sender)
        .add_attribute("effective_sender", sender)
        .add_attribute("proxy_action", action_name))
}

fn assert_creator(deps: Deps, sender: &Addr) -> ContractResult<()> {
    CREATOR
        .assert_owner(deps.storage, sender)
        .map_err(|_| ContractError::Unauthorized {})
}

/// What x/wasm knows about a prospective proxy. Best effort: an externally owned account
/// has no contract info at all.
enum ProxyKind {
    Account,
    Contract { code_id: u64, admin: Option<Addr> },
}

fn inspect_proxy(deps: Deps, proxy: &Addr) -> ProxyKind {
    match deps.querier.query_wasm_contract_info(proxy) {
        Ok(info) => ProxyKind::Contract {
            code_id: info.code_id,
            admin: info.admin,
        },
        Err(_) => ProxyKind::Account,
    }
}

fn add_trusted_proxy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proxy: String,
    require_immutable: bool,
) -> ContractResult<Response> {
    nonpayable(&info)?;
    assert_creator(deps.as_ref(), &info.sender)?;
    // While a handover is live the trusted set may only shrink: the incoming creator
    // must be able to rely on what they see before accepting.
    ensure!(
        !creator_transfer_is_live(deps.as_ref(), &env)?,
        ContractError::CreatorTransferPending {}
    );
    let proxy = deps.api.addr_validate(&proxy)?;
    ensure!(
        proxy != env.contract.address,
        ContractError::InvalidTrustedProxy {
            reason: "a collection cannot trust itself".to_string()
        }
    );
    ensure!(
        !TRUSTED_PROXIES.has(deps.storage, &proxy),
        ContractError::TrustedProxyAlreadyExists {
            proxy: proxy.to_string()
        }
    );
    ensure!(
        count_trusted_proxies(deps.storage) < MAX_TRUSTED_PROXIES,
        ContractError::TooManyTrustedProxies {
            max: MAX_TRUSTED_PROXIES
        }
    );
    // Always record what was registered; enforce immutability only when asked.
    let kind = inspect_proxy(deps.as_ref(), &proxy);
    let (kind_attr, code_id_attr, admin_attr) = match &kind {
        ProxyKind::Account => (
            "account".to_string(),
            "none".to_string(),
            "none".to_string(),
        ),
        ProxyKind::Contract { code_id, admin } => (
            "contract".to_string(),
            code_id.to_string(),
            admin.as_ref().map_or("none".to_string(), |a| a.to_string()),
        ),
    };
    if require_immutable {
        match &kind {
            ProxyKind::Contract { admin: None, .. } => {}
            ProxyKind::Contract { admin: Some(_), .. } => {
                return Err(ContractError::InvalidTrustedProxy {
                    reason: "proxy has a wasm admin and can be migrated".to_string(),
                });
            }
            ProxyKind::Account => {
                return Err(ContractError::InvalidTrustedProxy {
                    reason: "proxy is not a contract".to_string(),
                });
            }
        }
    }
    TRUSTED_PROXIES.save(deps.storage, &proxy, &cosmwasm_std::Empty {})?;
    Ok(Response::new().add_event(
        Event::new("trusted_proxy_added")
            .add_attribute("collection", env.contract.address)
            .add_attribute("proxy", proxy)
            .add_attribute("proxy_kind", kind_attr)
            .add_attribute("proxy_code_id", code_id_attr)
            .add_attribute("proxy_admin", admin_attr)
            .add_attribute("require_immutable", require_immutable.to_string())
            .add_attribute("by", info.sender),
    ))
}

fn remove_trusted_proxy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proxy: String,
) -> ContractResult<Response> {
    nonpayable(&info)?;
    assert_creator(deps.as_ref(), &info.sender)?;
    let proxy = deps.api.addr_validate(&proxy)?;
    ensure!(
        TRUSTED_PROXIES.has(deps.storage, &proxy),
        ContractError::TrustedProxyNotFound {
            proxy: proxy.to_string()
        }
    );
    TRUSTED_PROXIES.remove(deps.storage, &proxy);
    Ok(Response::new().add_event(
        Event::new("trusted_proxy_removed")
            .add_attribute("collection", env.contract.address)
            .add_attribute("proxy", proxy)
            .add_attribute("by", info.sender),
    ))
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(deps: Deps, env: Env, msg: ProxyableQueryMsg) -> StdResult<Binary> {
    match msg {
        ProxyableQueryMsg::Proxy(ProxyQueryMsg::GetTrustedProxies {}) => {
            to_json_binary(&list_trusted_proxies(deps.storage)?)
        }
        ProxyableQueryMsg::Base(base) => asset_base::query(deps, env, base),
    }
}

/// Migrate from a base `asset` contract or an earlier `asset-proxyable` version. The stored
/// cw2 identity is checked first because cw721's migrate rewrites it unconditionally.
/// The trusted-proxy map is cleared when either:
/// - the source is the base contract (a map does not empty itself on a code migration, and
///   dormant entries from an earlier proxyable life must not wake up), or
/// - the migration changed the creator (`WithUpdate { creator: Some(..) }` rotates the role
///   directly through cw721, bypassing the `AcceptOwnership` handover path, and must apply
///   the same rule: a new creator starts with no trusted proxies).
#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(mut deps: DepsMut, env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    let stored = cw2::get_contract_version(deps.storage)?;
    if !MIGRATABLE_FROM.contains(&(stored.contract.as_str(), stored.version.as_str())) {
        return Err(ContractError::InvalidMigration {
            contract: stored.contract,
            version: stored.version,
        });
    }
    let previous_creator = CREATOR.item.may_load(deps.storage)?.and_then(|o| o.owner);

    let contract: BaseContract<'static> = DefaultAssetContract::default();
    let response = contract.migrate(deps.branch(), env, msg, CONTRACT_NAME, CONTRACT_VERSION)?;

    let current_creator = CREATOR.item.may_load(deps.storage)?.and_then(|o| o.owner);
    let creator_changed = previous_creator != current_creator;
    let cleared = if stored.contract != CONTRACT_NAME || creator_changed {
        clear_trusted_proxies(deps.storage)?
    } else {
        0
    };
    Ok(response
        .add_attribute("from_contract", stored.contract)
        .add_attribute("creator_changed", creator_changed.to_string())
        .add_attribute("trusted_proxies_cleared", cleared.to_string()))
}

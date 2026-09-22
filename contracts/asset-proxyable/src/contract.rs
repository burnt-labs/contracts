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

/// `(cw2 name, cw2 version)` pairs this code can be migrated from. Extend when releasing.
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
        ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy { proxy }) => {
            add_trusted_proxy(deps, env, info, proxy)
        }
        ProxyableExecuteMsg::Proxy(ProxyMsg::RemoveTrustedProxy { proxy }) => {
            remove_trusted_proxy(deps, env, info, proxy)
        }
        ProxyableExecuteMsg::Base(base) => {
            // Proxy trust must always stay revocable: the creator cannot walk away while
            // any proxy is registered.
            if matches!(
                base,
                BaseExecuteMsg::UpdateCreatorOwnership(Action::RenounceOwnership)
            ) && has_trusted_proxies(deps.storage)
            {
                return Err(ContractError::TrustedProxiesNotEmpty {});
            }
            Ok(asset_base::execute(deps, env, info, base)?)
        }
    }
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

fn add_trusted_proxy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proxy: String,
) -> ContractResult<Response> {
    nonpayable(&info)?;
    assert_creator(deps.as_ref(), &info.sender)?;
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
    TRUSTED_PROXIES.save(deps.storage, &proxy, &cosmwasm_std::Empty {})?;
    Ok(Response::new().add_event(
        Event::new("trusted_proxy_added")
            .add_attribute("collection", env.contract.address)
            .add_attribute("proxy", proxy)
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
/// Coming from `asset`, the trusted-proxy map is cleared: a map does not empty itself on a
/// code migration, and dormant entries from an earlier proxyable life must not wake up.
#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    let stored = cw2::get_contract_version(deps.storage)?;
    if !MIGRATABLE_FROM.contains(&(stored.contract.as_str(), stored.version.as_str())) {
        return Err(ContractError::InvalidMigration {
            contract: stored.contract,
            version: stored.version,
        });
    }

    let mut cleared = 0;
    if stored.contract != CONTRACT_NAME {
        cleared = clear_trusted_proxies(deps.storage)?;
    }

    let contract: BaseContract<'static> = DefaultAssetContract::default();
    let response = contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(response
        .add_attribute("from_contract", stored.contract)
        .add_attribute("trusted_proxies_cleared", cleared.to_string()))
}

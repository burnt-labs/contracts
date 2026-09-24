use std::collections::BTreeSet;

use asset_proxyable::msg::{ProxyMsg, ProxyableExecuteMsg};
use cosmwasm_std::{
    Addr, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Response, StdResult, WasmMsg,
    ensure, to_json_binary,
};
use cw_utils::nonpayable;

use crate::{
    CONTRACT_NAME, CONTRACT_VERSION,
    error::{ContractError, ContractResult},
    msg::{
        ApprovalAction, ConfigResponse, ExecuteMsg, InstantiateMsg, IsAllowedResponse, ProxyAction,
        QueryMsg,
    },
    policy,
    state::{
        ALLOWED_OPERATORS, COLLECTIONS, CONFIG, Config, MAX_ALLOWED_OPERATORS,
        MAX_APPROVAL_CAP_SECONDS, MAX_COLLECTIONS, count_collections, list_collections,
        list_operators,
    },
};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    nonpayable(&info)?;
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if let Some(cap) = msg.max_approval_seconds {
        ensure!(
            cap > 0 && cap <= MAX_APPROVAL_CAP_SECONDS,
            ContractError::InvalidConfig {
                reason: format!(
                    "max_approval_seconds must be within 1..={MAX_APPROVAL_CAP_SECONDS}"
                )
            }
        );
    }
    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        max_approval_seconds: msg.max_approval_seconds,
    };
    // a self-administered proxy (possible with Instantiate2) could never be managed again
    ensure!(
        config.admin != env.contract.address,
        ContractError::SelfTarget {}
    );
    CONFIG.save(deps.storage, &config)?;

    ensure!(
        !msg.allowed_operators.is_empty(),
        ContractError::InvalidConfig {
            reason: "allowed_operators must not be empty".to_string()
        }
    );
    let mut operators = BTreeSet::new();
    for op in &msg.allowed_operators {
        operators.insert(deps.api.addr_validate(op)?);
    }
    ensure!(
        operators.len() <= MAX_ALLOWED_OPERATORS,
        ContractError::TooManyOperators {
            max: MAX_ALLOWED_OPERATORS
        }
    );
    for op in &operators {
        ensure!(*op != env.contract.address, ContractError::SelfTarget {});
        ALLOWED_OPERATORS.save(deps.storage, op, &Empty {})?;
    }

    let mut collections = BTreeSet::new();
    for c in &msg.collections {
        collections.insert(deps.api.addr_validate(c)?);
    }
    ensure!(
        collections.len() <= MAX_COLLECTIONS,
        ContractError::TooManyCollections {
            max: MAX_COLLECTIONS
        }
    );
    for c in &collections {
        ensure!(*c != env.contract.address, ContractError::SelfTarget {});
        COLLECTIONS.save(deps.storage, c, &Empty {})?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", config.admin)
        .add_attribute("operators", operators.len().to_string())
        .add_attribute("collections", collections.len().to_string()))
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    // The proxy never holds funds: every message is non-payable.
    nonpayable(&info)?;
    match msg {
        ExecuteMsg::SponsoredBurn {
            collection,
            token_id,
        } => sponsored(
            deps,
            env,
            info,
            collection,
            ProxyAction::Burn { token_id },
            "sponsored_burn",
        ),
        ExecuteMsg::SponsoredApproval { collection, action } => sponsored(
            deps,
            env,
            info,
            collection,
            ApprovalAction::into(action),
            "sponsored_approval",
        ),
        ExecuteMsg::AddCollection { collection } => add_collection(deps, env, info, collection),
        ExecuteMsg::RemoveCollection { collection } => remove_collection(deps, info, collection),
        ExecuteMsg::RemoveAllowedOperator { operator } => remove_operator(deps, info, operator),
        ExecuteMsg::UpdateAdmin { admin } => update_admin(deps, env, info, admin),
    }
}

/// Relay `action` to `collection` with `info.sender` as the effective sender.
fn sponsored(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: String,
    action: ProxyAction,
    action_name: &str,
) -> ContractResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let collection = deps.api.addr_validate(&collection)?;
    policy::check(deps.storage, &env, &config, &collection, &action)?;
    let proxy_action = match &action {
        ProxyAction::Burn { .. } => "burn",
        ProxyAction::ApproveAll { .. } => "approve_all",
        ProxyAction::RevokeAll { .. } => "revoke_all",
    };

    // The one place the envelope is built: the sender is always our caller.
    let envelope = ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
        sender: info.sender.to_string(),
        action,
    });
    Ok(Response::new()
        .add_message(WasmMsg::Execute {
            contract_addr: collection.to_string(),
            msg: to_json_binary(&envelope)?,
            funds: vec![],
        })
        .add_attribute("action", action_name)
        .add_attribute("proxy_action", proxy_action)
        .add_attribute("collection", collection)
        .add_attribute("sender", info.sender))
}

fn assert_admin(deps: Deps, sender: &Addr) -> ContractResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    ensure!(config.admin == *sender, ContractError::Unauthorized {});
    Ok(config)
}

fn add_collection(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: String,
) -> ContractResult<Response> {
    assert_admin(deps.as_ref(), &info.sender)?;
    let collection = deps.api.addr_validate(&collection)?;
    ensure!(
        collection != env.contract.address,
        ContractError::SelfTarget {}
    );
    ensure!(
        !COLLECTIONS.has(deps.storage, &collection),
        ContractError::CollectionAlreadyAllowed {
            collection: collection.to_string()
        }
    );
    ensure!(
        count_collections(deps.storage) < MAX_COLLECTIONS,
        ContractError::TooManyCollections {
            max: MAX_COLLECTIONS
        }
    );
    COLLECTIONS.save(deps.storage, &collection, &Empty {})?;
    Ok(Response::new().add_event(
        Event::new("collection_added")
            .add_attribute("collection", collection)
            .add_attribute("by", info.sender),
    ))
}

fn remove_collection(
    deps: DepsMut,
    info: MessageInfo,
    collection: String,
) -> ContractResult<Response> {
    assert_admin(deps.as_ref(), &info.sender)?;
    let collection = deps.api.addr_validate(&collection)?;
    ensure!(
        COLLECTIONS.has(deps.storage, &collection),
        ContractError::CollectionNotAllowed {
            collection: collection.to_string()
        }
    );
    COLLECTIONS.remove(deps.storage, &collection);
    Ok(Response::new().add_event(
        Event::new("collection_removed")
            .add_attribute("collection", collection)
            .add_attribute("by", info.sender),
    ))
}

fn remove_operator(deps: DepsMut, info: MessageInfo, operator: String) -> ContractResult<Response> {
    assert_admin(deps.as_ref(), &info.sender)?;
    let operator = deps.api.addr_validate(&operator)?;
    ensure!(
        ALLOWED_OPERATORS.has(deps.storage, &operator),
        ContractError::OperatorNotFound {
            operator: operator.to_string()
        }
    );
    ALLOWED_OPERATORS.remove(deps.storage, &operator);
    Ok(Response::new().add_event(
        Event::new("operator_removed")
            .add_attribute("operator", operator)
            .add_attribute("by", info.sender),
    ))
}

fn update_admin(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    admin: String,
) -> ContractResult<Response> {
    let mut config = assert_admin(deps.as_ref(), &info.sender)?;
    let previous = config.admin.clone();
    let admin = deps.api.addr_validate(&admin)?;
    // handing administration to the proxy itself would orphan the allowlist for good
    ensure!(admin != env.contract.address, ContractError::SelfTarget {});
    config.admin = admin;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_event(
        Event::new("admin_updated")
            .add_attribute("previous", previous)
            .add_attribute("admin", config.admin),
    ))
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => {
            let config = CONFIG.load(deps.storage)?;
            to_json_binary(&ConfigResponse {
                admin: config.admin,
                allowed_operators: list_operators(deps.storage)?,
                max_approval_seconds: config.max_approval_seconds,
            })
        }
        QueryMsg::Collections { start_after, limit } => {
            let start = start_after
                .map(|s| deps.api.addr_validate(&s))
                .transpose()?;
            to_json_binary(&list_collections(deps.storage, start.as_ref(), limit)?)
        }
        QueryMsg::IsAllowed { collection, action } => {
            let config = CONFIG.load(deps.storage)?;
            let result = deps
                .api
                .addr_validate(&collection)
                .map_err(ContractError::from)
                .and_then(|c| policy::check(deps.storage, &env, &config, &c, &action));
            to_json_binary(&IsAllowedResponse {
                allowed: result.is_ok(),
                reason: result.err().map(|e| e.to_string()),
            })
        }
    }
}

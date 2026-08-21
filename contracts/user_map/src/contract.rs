use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::InstantiateMsg;
use crate::msg::{ExecuteMsg, MigrateMsg, QueryMsg};
use crate::state::{DEFAULT_QUERY_LIMIT, MAX_QUERY_LIMIT, MAX_VALUE_LEN, USER_MAP};
use crate::{CONTRACT_NAME, CONTRACT_VERSION};
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult,
};
use cw_storage_plus::Bound;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    Ok(Response::default())
}
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Update { value } => {
            // reject oversized values before parsing to avoid a parse-time DoS
            if value.len() > MAX_VALUE_LEN {
                return Err(ContractError::ValueTooLong(MAX_VALUE_LEN));
            }

            // validate JSON
            serde_json::from_str::<serde_json::Value>(&value)?;

            USER_MAP.save(deps.storage, info.sender, &value)?;
            Ok(Response::default())
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetValueByUser { address } => {
            let value = USER_MAP.load(deps.storage, address)?;
            to_json_binary(&value)
        }
        QueryMsg::GetUsers { start_after, limit } => {
            let limit = clamp_query_limit(limit);
            let start = addr_bound(deps, start_after)?;
            let addrs: StdResult<Vec<Addr>> = USER_MAP
                .keys(deps.storage, start, None, Order::Ascending)
                .take(limit)
                .collect();
            to_json_binary(&addrs?)
        }
        QueryMsg::GetMap { start_after, limit } => {
            let limit = clamp_query_limit(limit);
            let start = addr_bound(deps, start_after)?;
            let response: StdResult<Vec<(Addr, String)>> = USER_MAP
                .range(deps.storage, start, None, Order::Ascending)
                .take(limit)
                .collect();
            to_json_binary(&response?)
        }
    }
}

/// Clamp the caller-supplied page size to the configured bounds.
fn clamp_query_limit(limit: Option<u32>) -> usize {
    limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT) as usize
}

/// Validate an optional `start_after` address and convert it to an exclusive
/// range bound for paginated iteration.
fn addr_bound(deps: Deps, start_after: Option<String>) -> StdResult<Option<Bound<'static, Addr>>> {
    Ok(start_after
        .map(|s| deps.api.addr_validate(&s))
        .transpose()?
        .map(Bound::exclusive))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::from_json;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

    use crate::error::ContractError;
    use crate::state::MAX_VALUE_LEN;
    use crate::CONTRACT_NAME;

    #[test]
    fn instantiate_sets_contract_version() {
        let mut deps = mock_dependencies();
        let creator = deps.api.addr_make("creator");
        instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&creator, &[]),
            InstantiateMsg {},
        )
        .unwrap();

        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(version.contract, CONTRACT_NAME);
        assert_eq!(version.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn update_rejects_value_exceeding_max_length() {
        let mut deps = mock_dependencies();
        let sender = deps.api.addr_make("sender");
        let info = message_info(&sender, &[]);

        // valid JSON, but total length exceeds the cap
        let too_long = "a".repeat(MAX_VALUE_LEN + 1);
        let value = format!("{{\"k\":\"{too_long}\"}}");

        let err = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Update { value },
        )
        .unwrap_err();

        assert!(matches!(err, ContractError::ValueTooLong(_)), "got {err:?}");
    }

    #[test]
    fn get_users_paginates_with_limit() {
        let mut deps = mock_dependencies();
        for i in 1..=3u8 {
            let addr = deps.api.addr_make(&format!("u{i}"));
            USER_MAP
                .save(deps.as_mut().storage, addr, &format!("\"{i}\""))
                .unwrap();
        }

        // page 1: limit 2 of 3 total
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetUsers {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();
        let page1: Vec<Addr> = from_json(res).unwrap();
        assert_eq!(page1.len(), 2);

        // page 2: start_after the last address of page 1
        let res2 = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetUsers {
                start_after: Some(page1[1].to_string()),
                limit: Some(2),
            },
        )
        .unwrap();
        let page2: Vec<Addr> = from_json(res2).unwrap();
        assert_eq!(page2.len(), 1);
    }

    #[test]
    fn get_map_paginates_with_limit() {
        let mut deps = mock_dependencies();
        for i in 1..=3u8 {
            let addr = deps.api.addr_make(&format!("u{i}"));
            USER_MAP
                .save(deps.as_mut().storage, addr, &format!("\"{i}\""))
                .unwrap();
        }

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMap {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();
        let page1: Vec<(Addr, String)> = from_json(res).unwrap();
        assert_eq!(page1.len(), 2);
    }
}

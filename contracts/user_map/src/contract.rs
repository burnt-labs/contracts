use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::InstantiateMsg;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::USER_MAP;
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdResult,
};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
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
        QueryMsg::GetUsers {} => {
            let mut addrs: Vec<Addr> = Vec::new();
            for addr in USER_MAP.keys(deps.storage, None, None, Order::Ascending) {
                addrs.push(addr?)
            }
            to_json_binary(&addrs)
        }
        QueryMsg::GetMap {} => {
            let mut response: Vec<(Addr, String)> = Vec::new();
            for item in USER_MAP.range(deps.storage, None, None, Order::Ascending) {
                let (key, value) = item?;
                response.push((key, value))
            }
            to_json_binary(&response)
        }
    }
}

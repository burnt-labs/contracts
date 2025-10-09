use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::InstantiateMsg;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{USER_MAP, OPACITY_VERIFIER};
use cosmwasm_std::{entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult};
use serde_json::Value;
use crate::error::ContractError::VerificationError;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    OPACITY_VERIFIER.save(deps.storage, &msg.opacity_verifier)?;
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
        ExecuteMsg::Update { message, signature } => {
            // validate JSON
            let value = serde_json::from_str::<Value>(&message)?;

            let verified: bool = deps.querier.query_wasm_smart(
                OPACITY_VERIFIER.load(deps.storage)?,
                &opacity_verifier::msg::QueryMsg::Verify {
                    message,
                    signature
                }
            )?;

            if verified {
                USER_MAP.save(deps.storage, info.sender, &value)?;
            } else {
                return Err(VerificationError)
            }

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
            let mut response: Vec<(Addr, Value)> = Vec::new();
            for item in USER_MAP.range(deps.storage, None, None, Order::Ascending) {
                let (key, value) = item?;
                response.push((key, value))
            }
            to_json_binary(&response)
        }
    }
}

use std::collections::HashMap;
use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::InstantiateMsg;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{CLAIM_VALUE_KEY, USER_MAP, VERIFICATION_ADDR};
use cosmwasm_std::{entry_point, to_json_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, WasmMsg};
use crate::error::ContractError::ClaimKeyInvalid;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(msg.verification_addr.as_str())?;
    VERIFICATION_ADDR.save(deps.storage, &msg.verification_addr)?;
    if msg.claim_key.is_empty() {
        return Err(ClaimKeyInvalid)
    }
    CLAIM_VALUE_KEY.save(deps.storage, &msg.claim_key)?;

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
            let context: HashMap<&str, String> = serde_json::from_str(&value.proof.claimInfo.context)?;
            let verified_value = match context.get(CLAIM_VALUE_KEY.load(deps.storage)?.as_str()) {
                Some(v) => v.to_string(),
                None => return Err(ContractError::JSONKeyMissing {}),
            };

            USER_MAP.save(deps.storage, info.sender, &verified_value)?;
            Ok(Response::default().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: VERIFICATION_ADDR.load(deps.storage)?.into_string(),
                msg: to_json_binary(&reclaim_xion::msg::ExecuteMsg::VerifyProof(value))?,
                funds: vec![],
            })))
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

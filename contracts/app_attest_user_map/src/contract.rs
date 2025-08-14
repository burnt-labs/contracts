use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::InstantiateMsg;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{UserStatus, APP_ATTEST_VERIFICATION_ADDR, APP_ID, USER_MAP};
use cosmwasm_std::{entry_point, from_json, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, QueryRequest, Response, StdResult, WasmQuery};
use crate::error::ContractError::InvalidAppId;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    APP_ATTEST_VERIFICATION_ADDR.save(deps.storage, &msg.verification_addr)?;
    APP_ID.save(deps.storage, &msg.app_id)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("verification_addr", msg.verification_addr.to_string()))
}
#[entry_point]
pub fn execute(
    deps: DepsMut,
    _: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Update { attestation } => {
            // require app id to match
            if attestation.app_id != APP_ID.load(deps.storage)? {
                return Err(InvalidAppId)
            }

            // get the verification contract address
            let verification_addr = APP_ATTEST_VERIFICATION_ADDR.load(deps.storage)?;

            // create the verification message
            let query_msg = ios_app_attest::msg::QueryMsg::VerifyAttestation(attestation.clone());

            // query the verification contract, fails if verification fails
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: verification_addr.to_string(),
                msg: to_json_binary(&query_msg)?,
            }))?;

            // deserialize the challenge into the expected type
            let user_status: UserStatus = from_json(&attestation.challenge)?;

            // save the data to the user's record
            USER_MAP.save(deps.storage, info.sender, &user_status)?;

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
            let mut response: Vec<(Addr, UserStatus)> = Vec::new();
            for item in USER_MAP.range(deps.storage, None, None, Order::Ascending) {
                let (key, value) = item?;
                response.push((key, value))
            }
            to_json_binary(&response)
        }
    }
}

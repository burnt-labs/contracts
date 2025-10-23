use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::InstantiateMsg;
use crate::msg::{ExecuteMsg, QueryMsg};
use crate::state::{APP_ID, USER_MAP};
use cosmwasm_std::{entry_point, to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult};
use crate::error::ContractError::InvalidAppId;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // APP_ATTEST_VERIFICATION_ADDR.save(deps.storage, &msg.verification_addr)?;
    APP_ID.save(deps.storage, &msg.app_id)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
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
            // let verification_addr = APP_ATTEST_VERIFICATION_ADDR.load(deps.storage)?;

            // create the verification message
            ios_app_attest::verify_attestation(attestation.app_id, attestation.key_id, attestation.challenge.clone(), attestation.cbor_data, env.block.time.seconds() as i64, attestation.dev_env)?;

            // save the data to the user's record
            USER_MAP.save(deps.storage, info.sender, &attestation.challenge)?;

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
            let mut response: Vec<(Addr, Binary)> = Vec::new();
            for item in USER_MAP.range(deps.storage, None, None, Order::Ascending) {
                let (key, value) = item?;
                response.push((key, value))
            }
            to_json_binary(&response)
        }
    }
}

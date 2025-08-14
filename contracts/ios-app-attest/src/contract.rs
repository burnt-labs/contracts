use cosmwasm_std::StdError;
use cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use crate::error::{ContractError, ContractResult};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::query::verify_attestation;

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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    Ok(Response::new()
        .add_attribute("method", "execute")
        .add_attribute("sender", info.sender))
}

#[entry_point]
pub fn query(_deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VerifyAttestation { 0: attestation } => {
            match verify_attestation(attestation.app_id, attestation.key_id, attestation.challenge, attestation.cbor_data, env.block.time.seconds() as i64, attestation.dev_env) {
                Ok(attestation) => to_json_binary(&true),
                Err(e) => Err(StdError::generic_err(e.to_string())),
            }
        }
    }
}


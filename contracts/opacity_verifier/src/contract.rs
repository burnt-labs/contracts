use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Response, StdResult};
use crate::error::{ContractError, ContractResult};
use crate::msg::{QueryMsg, ExecuteMsg, InstantiateMsg};
use crate::{query, CONTRACT_NAME, CONTRACT_VERSION};
use crate::state::{ADMIN, VERIFICATION_KEY_ALLOW_LIST};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADMIN.save(deps.storage, &msg.admin)?;
    for key in msg.allow_list {
        VERIFICATION_KEY_ALLOW_LIST.save(deps.storage, key, &Empty{})?;
    }
    Ok(Response::new().add_event(Event::new("create_opacity_verifier").add_attributes( vec![
        ("admin", msg.admin.into_string()),
    ])))
}


#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => {
            ADMIN.save(deps.storage, &admin)?;
        }
        ExecuteMsg::UpdateAllowList { keys  } => {
            VERIFICATION_KEY_ALLOW_LIST.clear(deps.storage);
            for key in keys {
                VERIFICATION_KEY_ALLOW_LIST.save(deps.storage, key, &Empty{})?;
            }
        }
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Verify { signature, message } => to_json_binary(
            &query::verify_query(deps.storage, signature, message)?),
        QueryMsg::VerificationKeys {} => to_json_binary(&query::verification_keys(deps.storage)?),
        QueryMsg::Admin {} => to_json_binary(&query::admin(deps.storage)?),
    }
}
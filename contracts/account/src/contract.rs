pub use absacc::AccountSudoMsg;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::execute::{add_auth_method, assert_self, remove_auth_method};
use crate::msg::{ExecuteMsg, MigrateMsg};
use crate::{
    error::ContractResult,
    execute,
    msg::{InstantiateMsg, QueryMsg},
    query, CONTRACT_NAME, CONTRACT_VERSION,
};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::init(deps, env, &mut msg.authenticator.clone())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: AccountSudoMsg) -> ContractResult<Response> {
    match msg {
        AccountSudoMsg::BeforeTx {
            tx_bytes,
            cred_bytes,
            simulate,
            ..
        } => {
            let cred_bytes = cred_bytes.ok_or(ContractError::EmptySignature)?;
            execute::before_tx(
                deps.as_ref(),
                &env,
                &Binary::from(tx_bytes.as_slice()),
                Some(Binary::from(cred_bytes.as_slice())).as_ref(),
                simulate,
            )
        }
        AccountSudoMsg::AfterTx { .. } => execute::after_tx(),
    }
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;
    let mut owned_msg = msg.clone();
    match &mut owned_msg {
        ExecuteMsg::AddAuthMethod { add_authenticator } => {
            add_auth_method(deps, &env, add_authenticator)
        }
        ExecuteMsg::RemoveAuthMethod { id } => remove_auth_method(deps, env, *id),
    }
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AuthenticatorIDs {} => to_json_binary(&query::authenticator_ids(deps.storage)?),
        QueryMsg::AuthenticatorByID { id } => {
            to_json_binary(&query::authenticator_by_id(deps.storage, id)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // No state migrations performed, just returned a Response
    Ok(Response::default())
}

use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use absacc::AccountSudoMsg;

use crate::error::ContractError;
use crate::execute::{add_auth_method, assert_self, remove_auth_method};
use crate::msg::{ExecuteMsg, MigrateMsg};
use crate::proto::XionCustomQuery;
use crate::{
    error::ContractResult,
    execute,
    msg::{InstantiateMsg, QueryMsg},
    query, CONTRACT_NAME, CONTRACT_VERSION,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut<XionCustomQuery>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::init(deps, env, &mut msg.authenticator.clone())
}

#[entry_point]
pub fn sudo(
    deps: DepsMut<XionCustomQuery>,
    env: Env,
    msg: AccountSudoMsg,
) -> ContractResult<Response> {
    match msg {
        AccountSudoMsg::BeforeTx {
            tx_bytes,
            cred_bytes,
            simulate,
            ..
        } => execute::before_tx(
            deps.as_ref(),
            &env,
            &tx_bytes,
            cred_bytes.as_ref(),
            simulate,
        ),
        AccountSudoMsg::AfterTx { .. } => execute::after_tx(),
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut<XionCustomQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;
    let mut owned_msg = msg.clone();
    match &mut owned_msg {
        ExecuteMsg::AddAuthMethod { add_authenticator } => {
            add_auth_method(deps, env, add_authenticator)
        }
        ExecuteMsg::RemoveAuthMethod { id } => remove_auth_method(deps, env, *id),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AuthenticatorIDs {} => to_binary(&query::authenticator_ids(deps.storage)?),
        QueryMsg::AuthenticatorByID { id } => {
            to_binary(&query::authenticator_by_id(deps.storage, id)?)
        }
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // No state migrations performed, just returned a Response
    Ok(Response::default())
}

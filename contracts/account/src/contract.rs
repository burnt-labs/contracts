use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, AnyMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

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

/// Any contract must implement this sudo message (both variants) in order to
/// qualify as an abstract account.
#[cw_serde]
pub enum AccountSudoMsg {
    /// Called by the AnteHandler's BeforeTxDecorator before a tx is executed.
    BeforeTx {
        /// Messages the tx contains
        msgs: Vec<AnyMsg>,

        /// The tx serialized into binary format.
        ///
        /// If the tx authentication requires a signature, this is the bytes to
        /// be signed.
        tx_bytes: Binary,

        /// The credential to prove this tx is authenticated.
        ///
        /// This is taken from the tx's "signature" field, but in the case of
        /// AbstractAccounts, this is not necessarily a cryptographic signature.
        /// The contract is free to interpret this as any data type.
        cred_bytes: Option<Binary>,

        /// Whether the tx is being run in the simulation mode.
        simulate: bool,
    },

    /// Called by the PostHandler's AfterTxDecorator after the tx is executed.
    AfterTx {
        /// Whether the tx is being run in the simulation mode.
        simulate: bool,
    },
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

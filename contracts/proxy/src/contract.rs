use crate::error::ContractResult;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::{execute, CONTRACT_NAME, CONTRACT_VERSION};
use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::init(deps, info, msg.admin, msg.code_ids)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::ProxyMsgs { msgs } => execute::proxy_msgs(deps, info, msgs),
        ExecuteMsg::AddCodeIDs { code_ids } => execute::add_code_ids(deps, info, code_ids),
        ExecuteMsg::RemoveCodeIDs { code_ids } => execute::remove_code_ids(deps, info, code_ids),
        ExecuteMsg::UpdateAdmin { new_admin } => execute::update_admin(deps, info, new_admin),
    }
}

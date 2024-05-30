use cosmos_sdk_proto::cosmos::authz::v1beta1::MsgGrant;
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgGrantAllowance;
use cosmwasm_std::{entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, CosmosMsg};
use crate::error::ContractResult;
use crate::execute::deploy_fee_grant;
use crate::msg::ExecuteMsg;

// #[entry_point]
// pub fn instantiate(
//     deps: DepsMut<XionCustomQuery>,
//     env: Env,
//     _info: MessageInfo,
//     msg: InstantiateMsg,
// ) -> ContractResult<Response> {
//     cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
//     execute::init(deps, env, &mut msg.authenticator.clone())
// }

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    
    match msg { 
        ExecuteMsg::DeployFeeGrant { authz_granter, authz_grantee, msg_type_url, authorization } => deploy_fee_grant(deps, env, info, authz_granter, authz_grantee, msg_type_url, authorization),
    }
}

// #[entry_point]
// pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     match msg {
//     }
// }
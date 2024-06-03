use crate::error::ContractResult;
use crate::execute::deploy_fee_grant;
use crate::msg::ExecuteMsg;
use crate::proto::XionCustomQuery;
use cosmwasm_std::{entry_point, DepsMut, Env, MessageInfo, Response};

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
    deps: DepsMut<XionCustomQuery>,
    env: Env,
    _: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::DeployFeeGrant {
            authz_granter,
            authz_grantee,
            authorization,
        } => deploy_fee_grant(deps, env, authz_granter, authz_grantee, authorization),
    }
}

// #[entry_point]
// pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     match msg {
//     }
// }

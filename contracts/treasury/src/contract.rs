use crate::error::ContractResult;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::{execute, query, CONTRACT_NAME, CONTRACT_VERSION};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::init(deps, info, msg.admin, msg.type_urls, msg.grant_configs)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::DeployFeeGrant {
            authz_granter,
            authz_grantee,
            msg_type_url,
        } => execute::deploy_fee_grant(deps, env, authz_granter, authz_grantee, msg_type_url),
        ExecuteMsg::UpdateAdmin { new_admin } => execute::update_admin(deps, info, new_admin),
        ExecuteMsg::UpdateGrantConfig {
            msg_type_url,
            grant_config,
        } => execute::update_grant_config(deps, info, msg_type_url, grant_config),
        ExecuteMsg::RemoveGrantConfig { msg_type_url } => {
            execute::remove_grant_config(deps, info, msg_type_url)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GrantConfigByTypeURL { msg_type_url } => to_binary(
            &query::grant_config_by_type_url(deps.storage, msg_type_url)?,
        ),
        QueryMsg::GrantConfigTypeURLs {} => {
            to_binary(&query::grant_config_type_urls(deps.storage)?)
        }
    }
}

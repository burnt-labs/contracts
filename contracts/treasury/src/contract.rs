use crate::error::{ContractError, ContractResult};
use crate::execute::{revoke_allowance, update_fee_config, update_params, withdraw_coins};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::{execute, query, CONTRACT_NAME, CONTRACT_VERSION};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // Validate the admin address
    let admin_addr = if let Some(addr) = msg.admin {
        deps.api.addr_validate(addr.as_str())?
    } else {
        return Err(ContractError::Unauthorized);
    };
    execute::init(
        deps,
        info,
        Some(admin_addr),
        msg.type_urls,
        msg.grant_configs,
        msg.fee_config,
        msg.params,
    )
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
        } => execute::deploy_fee_grant(deps, env, authz_granter, authz_grantee),
        ExecuteMsg::ProposeAdmin { new_admin } => {
            execute::propose_admin(deps, info, new_admin.into_string())
        }
        ExecuteMsg::AcceptAdmin {} => execute::accept_admin(deps, info),
        ExecuteMsg::CancelProposedAdmin {} => execute::cancel_proposed_admin(deps, info),
        ExecuteMsg::UpdateGrantConfig {
            msg_type_url,
            grant_config,
        } => execute::update_grant_config(deps, info, msg_type_url, grant_config),
        ExecuteMsg::RemoveGrantConfig { msg_type_url } => {
            execute::remove_grant_config(deps, info, msg_type_url)
        }
        ExecuteMsg::UpdateFeeConfig { fee_config } => update_fee_config(deps, info, fee_config),
        ExecuteMsg::RevokeAllowance { grantee } => revoke_allowance(deps, env, info, grantee),
        ExecuteMsg::UpdateParams { params } => update_params(deps, info, params),
        ExecuteMsg::Withdraw { coins } => withdraw_coins(deps, info, coins),
        ExecuteMsg::Migrate {
            new_code_id,
            migrate_msg,
        } => execute::migrate(deps, env, info, new_code_id, migrate_msg),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GrantConfigByTypeUrl { msg_type_url } => to_json_binary(
            &query::grant_config_by_type_url(deps.storage, msg_type_url)?,
        ),
        QueryMsg::GrantConfigTypeUrls {} => {
            to_json_binary(&query::grant_config_type_urls(deps.storage))
        }
        QueryMsg::FeeConfig {} => to_json_binary(&query::fee_config(deps.storage)?),
        QueryMsg::Admin {} => to_json_binary(&query::admin(deps.storage)?),
        QueryMsg::PendingAdmin {} => to_json_binary(&query::pending_admin(deps.storage)?),
        QueryMsg::Params {} => to_json_binary(&query::params(deps.storage)?),
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // No state migrations performed, just returned a Response
    Ok(Response::default())
}

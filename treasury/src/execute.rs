use crate::error::ContractError::{
    AuthzGrantNoAuthorization, AuthzGrantNotFound, ConfigurationMismatch,
};
use crate::error::ContractResult;
use crate::grant::allowance::format_allowance;
use crate::grant::{Any, GrantConfig};
use crate::proto::XionCustomQuery;
use crate::state::{ADMIN, GRANT_CONFIGS};
use cosmos_sdk_proto::cosmos::authz::v1beta1::{QueryGrantsRequest, QueryGrantsResponse};
use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Env, Event, MessageInfo, Response};

pub fn init(
    deps: DepsMut<XionCustomQuery>,
    info: MessageInfo,
    admin: Option<Addr>,
    type_urls: Vec<String>,
    grant_configs: Vec<GrantConfig>,
) -> ContractResult<Response> {
    let treasury_admin = match admin {
        None => info.sender,
        Some(adm) => adm,
    };
    ADMIN.save(deps.storage, &treasury_admin)?;

    if type_urls.len().ne(&grant_configs.len()) {
        return Err(ConfigurationMismatch);
    }

    for i in 0..type_urls.len() {
        GRANT_CONFIGS.save(deps.storage, type_urls[i].clone(), &grant_configs[i])?;
    }

    Ok(Response::new().add_event(
        Event::new("create_treasury_instance")
            .add_attributes(vec![("admin", treasury_admin.into_string())]),
    ))
}

pub fn deploy_fee_grant(
    deps: DepsMut<XionCustomQuery>,
    env: Env,
    authz_granter: Addr,
    authz_grantee: Addr,
    authorization: Any,
) -> ContractResult<Response> {
    // check if grant exists in patterns on contract
    let grant_config = GRANT_CONFIGS.load(deps.storage, authorization.msg_type_url.clone())?;

    // check if grant exists on chain
    let query_msg = QueryGrantsRequest {
        granter: authz_granter.to_string(),
        grantee: authz_grantee.to_string(),
        msg_type_url: authorization.msg_type_url,
        pagination: None,
    };
    let grants = deps
        .querier
        .query::<QueryGrantsResponse>(&query_msg.into())?;
    // grant queries with a granter, grantee and type_url should always result
    // in only one result
    if grants.grants.len() == 0 {
        return Err(AuthzGrantNotFound);
    }
    let grant = grants.grants[0].clone();
    let auth_any: Any = grant.authorization.ok_or(AuthzGrantNoAuthorization)?.into();
    if grant_config.authorization.ne(&auth_any) {
        return Err(AuthzGrantNoAuthorization);
    }
    // todo: do we allow authorizations without expiry?

    // create feegrant, if needed
    match grant_config.allowance {
        None => Ok(Response::new()),
        // allowance should be stored as a prost proto from the feegrant definition
        Some(allowance) => {
            let formatted_allowance = format_allowance(
                allowance,
                env.contract.address.clone(),
                authz_grantee.clone(),
                grant.expiration,
            )?;
            let feegrant_msg = cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgGrantAllowance {
                granter: env.contract.address.into_string(),
                grantee: authz_grantee.into_string(),
                allowance: Some(formatted_allowance.into()),
            };
            let feegrant_msg_bz = cosmwasm_std::to_binary(&feegrant_msg)?;
            let cosmos_msg = CosmosMsg::Stargate {
                type_url: "/cosmos.auth.v1beta1.Msg/MsgGrantAllowance".to_string(),
                value: feegrant_msg_bz,
            };
            Ok(Response::new().add_message(cosmos_msg))
        }
    }
}

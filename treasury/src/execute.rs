use crate::error::ContractError::{
    AuthzGrantMistmatch, AuthzGrantNoAuthorization, AuthzGrantNotFound, ConfigurationMismatch,
    Unauthorized,
};
use crate::error::ContractResult;
use crate::grant::allowance::format_allowance;
use crate::grant::{Any, GrantConfig};
use crate::state::{ADMIN, GRANT_CONFIGS};
use cosmos_sdk_proto::cosmos::authz::v1beta1::{QueryGrantsRequest, QueryGrantsResponse};
use cosmos_sdk_proto::traits::MessageExt;
use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Env, Event, MessageInfo, Response};

pub fn init(
    deps: DepsMut,
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

pub fn update_admin(deps: DepsMut, info: MessageInfo, new_admin: Addr) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    ADMIN.save(deps.storage, &new_admin)?;

    Ok(
        Response::new().add_event(Event::new("updated_treasury_admin").add_attributes(vec![
            ("old admin", admin.into_string()),
            ("new admin", new_admin.into_string()),
        ])),
    )
}

pub fn update_grant_config(
    deps: DepsMut,
    info: MessageInfo,
    msg_type_url: String,
    grant_config: GrantConfig,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    let existed = GRANT_CONFIGS.has(deps.storage, msg_type_url.clone());

    GRANT_CONFIGS.save(deps.storage, msg_type_url.clone(), &grant_config)?;

    Ok(Response::new().add_event(
        Event::new("updated_treasury_grant_config").add_attributes(vec![
            ("msg type url", msg_type_url),
            ("overwritten", existed.to_string()),
        ]),
    ))
}

pub fn remove_grant_config(
    deps: DepsMut,
    info: MessageInfo,
    msg_type_url: String,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    GRANT_CONFIGS.remove(deps.storage, msg_type_url.clone());

    Ok(Response::new().add_event(
        Event::new("removed_treasury_grant_config")
            .add_attributes(vec![("msg type url", msg_type_url)]),
    ))
}

pub fn deploy_fee_grant(
    deps: DepsMut,
    env: Env,
    authz_granter: Addr,
    authz_grantee: Addr,
    msg_type_url: String,
) -> ContractResult<Response> {
    // check if grant exists in patterns on contract
    let grant_config = GRANT_CONFIGS.load(deps.storage, msg_type_url.clone())?;

    // check if grant exists on chain
    let query_msg = QueryGrantsRequest {
        granter: authz_granter.to_string(),
        grantee: authz_grantee.to_string(),
        msg_type_url: msg_type_url.clone(),
        pagination: None,
    };
    let grants =
        deps.querier
            .query::<QueryGrantsResponse>(&cosmwasm_std::QueryRequest::Stargate {
                path: "/cosmos.authz.v1beta1.Query/Grants".to_string(),
                data: query_msg.to_bytes().unwrap().into(),
            })?;
    // grant queries with a granter, grantee and type_url should always result
    // in only one result
    if grants.grants.is_empty() {
        return Err(AuthzGrantNotFound);
    }
    let grant = grants.grants[0].clone();
    let auth_any: Any = grant.authorization.ok_or(AuthzGrantNoAuthorization)?.into();
    if grant_config.authorization.ne(&auth_any) {
        return Err(AuthzGrantMistmatch);
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

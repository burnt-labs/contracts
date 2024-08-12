use cosmos_sdk_proto::cosmos::authz::v1beta1::{QueryGrantsRequest, QueryGrantsResponse};
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::QueryAllowanceRequest;
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::Timestamp;
use cosmwasm_std::{
    Addr, AnyMsg, Binary, CosmosMsg, DepsMut, Env, Event, MessageInfo, Order, Response,
};

use crate::error::ContractError::{
    AuthzGrantMismatch, AuthzGrantNotFound, ConfigurationMismatch, Unauthorized,
};
use crate::error::ContractResult;
use crate::grant::allowance::format_allowance;
use crate::grant::{FeeConfig, GrantConfig};
use crate::state::{ADMIN, FEE_CONFIG, GRANT_CONFIGS};

pub fn init(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<Addr>,
    type_urls: Vec<String>,
    grant_configs: Vec<GrantConfig>,
    fee_config: FeeConfig,
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

    FEE_CONFIG.save(deps.storage, &fee_config)?;

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

pub fn update_fee_config(
    deps: DepsMut,
    info: MessageInfo,
    fee_config: FeeConfig,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    FEE_CONFIG.save(deps.storage, &fee_config)?;

    Ok(Response::new().add_event(Event::new("updated_treasury_fee_config")))
}

pub fn deploy_fee_grant(
    deps: DepsMut,
    env: Env,
    authz_granter: Addr,
    authz_grantee: Addr,
) -> ContractResult<Response> {
    // iterate through all grant configs to validate user has correct permissions
    // we must iterate, because calling for the list of grants doesn't return msg_type_urls
    for key in GRANT_CONFIGS.keys(deps.storage, None, None, Order::Ascending) {
        let msg_type_url = key?;
        let grant_config = GRANT_CONFIGS.load(deps.storage, msg_type_url.clone())?;

        // check if grant exists on chain
        let authz_query_msg_bytes = QueryGrantsRequest {
            granter: authz_granter.to_string(),
            grantee: authz_grantee.to_string(),
            msg_type_url: msg_type_url.clone(),
            pagination: None,
        }
        .to_bytes()?;
        let authz_query_res = deps.querier.query_grpc(
            String::from("/cosmos.authz.v1beta1.Query/Grants"),
            Binary::new(authz_query_msg_bytes),
        )?;

        let response = QueryGrantsResponse::decode(authz_query_res.as_slice())?;
        let grants = response.grants;

        if grants.clone().is_empty() && !grant_config.optional {
            return Err(AuthzGrantNotFound { msg_type_url });
        } else {
            match grants.first() {
                None => return Err(AuthzGrantNotFound { msg_type_url }),
                Some(grant) => {
                    match grant.clone().authorization {
                        None => return Err(AuthzGrantNotFound { msg_type_url }),
                        Some(auth) => {
                            // the authorization must match the one in the config
                            if grant_config.authorization.ne(&auth.into()) {
                                return Err(AuthzGrantMismatch);
                            }
                        }
                    }
                }
            }
        }
    }
    // at this point, all the authz grants in the grant_config are verified

    let fee_config = FEE_CONFIG.load(deps.storage)?;
    // create feegrant, if needed
    match fee_config.allowance {
        // this treasury doesn't deploy any fees, and can return
        None => Ok(Response::new()),
        // allowance should be stored as a prost proto from the feegrant definition
        Some(allowance) => {
            // build the new allowance based on expiration
            let expiration = match fee_config.expiration {
                None => None,
                Some(seconds) => {
                    let expiration_time = env.block.time.plus_seconds(seconds as u64);
                    Some(Timestamp {
                        seconds: expiration_time.seconds() as i64,
                        nanos: expiration_time.subsec_nanos() as i32,
                    })
                }
            };

            let formatted_allowance = format_allowance(
                allowance,
                env.contract.address.clone(),
                authz_grantee.clone(),
                expiration,
            )?;
            let feegrant_msg_bytes =
                cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgGrantAllowance {
                    granter: env.contract.address.clone().into_string(),
                    grantee: authz_grantee.clone().into_string(),
                    allowance: Some(formatted_allowance.into()),
                }
                .to_bytes()?;
            let cosmos_feegrant_msg = CosmosMsg::Any(AnyMsg {
                type_url: "/cosmos.feegrant.v1beta1.MsgGrantAllowance".to_string(),
                value: feegrant_msg_bytes.into(),
            });

            // check to see if the user already has an existing feegrant
            let feegrant_query_msg_bytes = QueryAllowanceRequest {
                granter: env.contract.address.to_string(),
                grantee: authz_grantee.to_string(),
            }
            .to_bytes()?;
            let feegrant_query_res = deps
                .querier
                .query_grpc(
                    "/cosmos.feegrant.v1beta1.Query/Allowance".to_string(),
                    feegrant_query_msg_bytes.into(),
                )
                .unwrap_or_else(|_| Binary::default());

            let mut msgs: Vec<CosmosMsg> = Vec::new();
            if !feegrant_query_res.is_empty() {
                let feegrant_revoke_msg_bytes =
                    cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgRevokeAllowance {
                        granter: env.contract.address.clone().into_string(),
                        grantee: authz_grantee.clone().into_string(),
                    }
                    .to_bytes()?;
                let cosmos_revoke_msg = CosmosMsg::Any(AnyMsg {
                    type_url: "/cosmos.feegrant.v1beta1.MsgRevokeAllowance".to_string(),
                    value: feegrant_revoke_msg_bytes.into(),
                });
                msgs.push(cosmos_revoke_msg);
            }
            msgs.push(cosmos_feegrant_msg);
            Ok(Response::new().add_messages(msgs))
        }
    }
}

pub fn revoke_allowance(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    grantee: Addr,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    let feegrant_revoke_msg_bytes =
        cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgRevokeAllowance {
            granter: env.contract.address.into_string(),
            grantee: grantee.clone().into_string(),
        }
        .to_bytes()?;
    let cosmos_feegrant_revoke_msg = CosmosMsg::Any(AnyMsg {
        type_url: "/cosmos.feegrant.v1beta1.MsgRevokeAllowance".to_string(),
        value: feegrant_revoke_msg_bytes.into(),
    });

    Ok(Response::new()
        .add_message(cosmos_feegrant_revoke_msg)
        .add_event(
            Event::new("revoked_treasury_allowance")
                .add_attributes(vec![("grantee", grantee.into_string())]),
        ))
}

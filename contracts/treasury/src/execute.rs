use crate::error::ContractError::{
    self, AuthzGrantMismatch, AuthzGrantNoAuthorization, AuthzGrantNotFound, ConfigurationMismatch,
    Unauthorized,
};
use crate::error::ContractResult;
use crate::grant::allowance::format_allowance;
use crate::grant::GrantConfig;
use crate::state::{ADMIN, GRANT_CONFIGS};
use cosmos_sdk_proto::cosmos::authz::v1beta1::QueryGrantsRequest;
use cosmos_sdk_proto::tendermint::serializers::timestamp;
use cosmos_sdk_proto::traits::MessageExt;
use cosmwasm_std::{Addr, CosmosMsg, DepsMut, Env, Event, MessageInfo, Response};
use pbjson_types::Timestamp;
use serde::de::value::{Error, StringDeserializer};
use serde_json::Value;

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
    info: MessageInfo,
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
    let query_msg_bytes = match query_msg.to_bytes() {
        Ok(bz) => bz,
        Err(_) => {
            return Err(ContractError::Std(cosmwasm_std::StdError::SerializeErr {
                source_type: String::from("QueryGrantsRequest"),
                msg: "Unable to serialize QueryGrantsRequest".to_string(),
            }))
        }
    };
    let query_res = deps
        .querier
        .query::<Value>(&cosmwasm_std::QueryRequest::Stargate {
            path: "/cosmos.authz.v1beta1.Query/Grants".to_string(),
            data: query_msg_bytes.into(),
        })?;

    let grants = &query_res["grants"];
    // grant queries with a granter, grantee and type_url should always result
    // in only one result
    if !grants.is_array() {
        return Err(AuthzGrantNotFound);
    }
    let grant = grants[0].clone();
    if grant.is_null() {
        return Err(AuthzGrantNotFound);
    }

    let auth = &grant["authorization"];
    if auth.is_null() {
        return Err(AuthzGrantNoAuthorization);
    }

    if grant_config.authorization.ne(auth) {
        return Err(AuthzGrantMismatch);
    }

    // create feegrant, if needed
    match grant_config.allowance {
        None => Ok(Response::new()),
        // allowance should be stored as a prost proto from the feegrant definition
        Some(allowance) => {
            let grant_expiration =
                grant["expiration"].as_str().map(|t| {
                    match timestamp::deserialize(StringDeserializer::<Error>::new(t.to_string())) {
                        Ok(tm) => Timestamp {
                            seconds: tm.seconds,
                            nanos: tm.nanos,
                        },
                        Err(_) => Timestamp::default(),
                    }
                });

            let max_expiration = match grant_config.max_duration {
                None => None,
                Some(duration) => {
                    let max_timestamp = env.block.time.plus_seconds(duration as u64);
                    Some(Timestamp {
                        seconds: max_timestamp.seconds() as i64,
                        nanos: max_timestamp.nanos() as i32,
                    })
                }
            };

            let expiration = match grant_expiration {
                None => max_expiration,
                Some(grant_expiration) => match max_expiration {
                    None => Some(grant_expiration),
                    Some(max_expiration) => {
                        if max_expiration.seconds < grant_expiration.seconds {
                            Some(max_expiration)
                        } else {
                            Some(grant_expiration)
                        }
                    }
                },
            };

            let formatted_allowance = format_allowance(
                allowance,
                env.contract.address.clone(),
                authz_grantee.clone(),
                expiration,
            )?;
            let feegrant_msg = cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgGrantAllowance {
                granter: env.contract.address.into_string(),
                grantee: authz_grantee.into_string(),
                allowance: Some(formatted_allowance.into()),
            };
            let feegrant_msg_bytes = feegrant_msg.to_bytes()?;
            // todo: what if a feegrant already exists?
            let cosmos_msg = CosmosMsg::Stargate {
                type_url: "/cosmos.feegrant.v1beta1.MsgGrantAllowance".to_string(),
                value: feegrant_msg_bytes.into(),
            };
            Ok(Response::new().add_message(cosmos_msg))
        }
    }
}

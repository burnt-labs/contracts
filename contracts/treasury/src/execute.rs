use crate::error::ContractError::{
    AuthzGrantMismatch, AuthzGrantNotFound, ConfigurationMismatch, GrantConfigNotFound,
    Unauthorized,
};
use crate::error::ContractResult;
use crate::grant::allowance::format_allowance;
use crate::grant::{FeeConfig, GrantConfig};
use crate::state::{Params, ADMIN, FEE_CONFIG, GRANT_CONFIGS, PARAMS, PENDING_ADMIN};
use cosmos_sdk_proto::cosmos::authz::v1beta1::{QueryGrantsRequest, QueryGrantsResponse};
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::QueryAllowanceRequest;
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::Timestamp;
use cosmwasm_std::BankMsg::Send;
use cosmwasm_std::{
    Addr, AnyMsg, Binary, Coin, CosmosMsg, DepsMut, Env, Event, MessageInfo, Order, Response,
    WasmMsg,
};
use url::Url;

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

pub fn propose_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: String,
) -> ContractResult<Response> {
    // Load the current admin
    let admin = ADMIN.load(deps.storage)?;

    // Check if the caller is the current admin
    if admin != info.sender {
        return Err(Unauthorized);
    }

    // Validate the new admin address
    let validated_admin = deps.api.addr_validate(&new_admin)?;

    // Save the proposed new admin to PENDING_ADMIN
    PENDING_ADMIN.save(deps.storage, &validated_admin)?;

    Ok(
        Response::new().add_event(Event::new("proposed_new_admin").add_attributes(vec![
            ("proposed_admin", validated_admin.to_string()),
            ("proposer", admin.to_string()),
        ])),
    )
}

pub fn accept_admin(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    // Load the pending admin
    let pending_admin = PENDING_ADMIN.load(deps.storage)?;

    // Verify the sender is the pending admin
    if pending_admin != info.sender {
        return Err(Unauthorized);
    }

    // Update the ADMIN storage with the new admin
    ADMIN.save(deps.storage, &pending_admin)?;

    // Clear the PENDING_ADMIN
    PENDING_ADMIN.remove(deps.storage);

    Ok(Response::new().add_event(
        Event::new("accepted_new_admin")
            .add_attributes(vec![("new_admin", pending_admin.to_string())]),
    ))
}

pub fn cancel_proposed_admin(deps: DepsMut, info: MessageInfo) -> ContractResult<Response> {
    // Load the current admin
    let admin = ADMIN.load(deps.storage)?;

    // Check if the caller is the current admin
    if admin != info.sender {
        return Err(Unauthorized);
    }

    // Remove the pending admin
    PENDING_ADMIN.remove(deps.storage);

    Ok(Response::new().add_event(
        Event::new("cancelled_proposed_admin").add_attribute("action", "cancel_proposed_admin"),
    ))
}

pub fn migrate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_code_id: u64,
    migrate_msg: Binary,
) -> ContractResult<Response> {
    // Load the current admin
    let admin = ADMIN.load(deps.storage)?;

    // Check if the caller is the current admin
    if admin != info.sender {
        return Err(Unauthorized);
    }

    // this assumes that the contract's wasmd admin is itself
    let migrate_msg = CosmosMsg::Wasm(WasmMsg::Migrate {
        contract_addr: env.contract.address.into_string(),
        new_code_id,
        msg: migrate_msg,
    });

    Ok(Response::new()
        .add_event(Event::new("migrate_treasury_instance").add_attributes(vec![
            ("new_code_id", new_code_id.to_string()),
            ("admin", admin.to_string()),
        ]))
        .add_message(migrate_msg))
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
    // Check if the sender is the admin
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    // Validate that the key exists
    if !GRANT_CONFIGS.has(deps.storage, msg_type_url.clone()) {
        return Err(GrantConfigNotFound {
            type_url: msg_type_url,
        });
    }

    // Remove the grant config
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

pub fn update_params(deps: DepsMut, info: MessageInfo, params: Params) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }
    
    Url::parse(params.display_url.as_str())?;
    for url in params.redirect_urls.iter() {
        Url::parse(url.as_str())?;
    }
    
    Url::parse(params.icon_url.as_str())?;

    PARAMS.save(deps.storage, &params)?;

    Ok(Response::new().add_event(Event::new("updated_params")))
}

pub fn withdraw_coins(
    deps: DepsMut,
    info: MessageInfo,
    coins: Vec<Coin>,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    Ok(Response::new().add_message(Send {
        to_address: info.sender.into_string(),
        amount: coins,
    }))
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

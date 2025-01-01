use crate::error::ContractError::{
    AuthzGrantMismatch, AuthzGrantNotFound, ConfigurationMismatch, Unauthorized,
};
use crate::error::ContractResult;
use crate::grant::allowance::format_allowance;
use crate::grant::{FeeConfig, GrantConfig};
use crate::msg::UpdateGrant;
use crate::state::{Params, ADMIN, FEE_CONFIG, GRANT_CONFIGS, PARAMS};
use cosmos_sdk_proto::cosmos::authz::v1beta1::{QueryGrantsRequest, QueryGrantsResponse};
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::QueryAllowanceRequest;
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::Timestamp;
use cosmwasm_std::BankMsg::Send;
use cosmwasm_std::{
    Addr, AnyMsg, Binary, Coin, CosmosMsg, DepsMut, Empty, Env, Event, MessageInfo, Order, Response,
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

pub fn update_configs(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    authz_grants: Option<Vec<UpdateGrant>>,
    fee_grants: Option<Vec<FeeConfig>>,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(Unauthorized);
    }

    let mut res = Response::<Empty>::new();
    res = res.add_event(Event::new("update_configs"));

    if let Some(grants) = authz_grants {
        for grant in grants {
            let existed = GRANT_CONFIGS.has(deps.storage, grant.msg_type_url.clone());

            GRANT_CONFIGS.save(
                deps.storage,
                grant.msg_type_url.clone(),
                &grant.grant_config,
            )?;
            res = res.add_attributes(vec![
                ("msg type url", grant.msg_type_url),
                ("overwritten", existed.to_string()),
            ]);
        }
    }

    if let Some(fee_grants) = fee_grants {
        for config in fee_grants {
            FEE_CONFIG.save(deps.storage, &config)?;
        }
    }

    Ok(res)
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

pub fn update_params(deps: DepsMut, info: MessageInfo, params: Params) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if admin != info.sender {
        return Err(Unauthorized);
    }

    Url::parse(params.display_url.as_str())?;
    Url::parse(params.redirect_url.as_str())?;
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

#[cfg(test)]
mod tests {
    use crate::contract::execute;
    use crate::grant::Any;
    use crate::msg::ExecuteMsg;
    use crate::state::{ADMIN, FEE_CONFIG, GRANT_CONFIGS};
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::Addr;

    #[test]
    fn test_update_configs_with_json_input() {
        // Arrange: Set up environment and dependencies
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = message_info(&Addr::unchecked("admin"), &[]);

        // Mock admin address in storage
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked("admin"))
            .unwrap();

        // Example JSON string for ExecuteMsg::UpdateConfigs
        let json_msg = r#"
        {
            "update_configs": {
                "grants": [
                    {
                        "msg_type_url": "/cosmos.bank.v1.MsgSend",
                        "grant_config": {
                            "description": "Bank grant",
                            "authorization": {
                                "type_url": "/cosmos.authz.v1.GenericAuthorization",
                                "value": "CgRQYXk="
                            },
                            "optional": true
                        }
                    },
                    {
                        "msg_type_url": "/cosmos.staking.v1.MsgDelegate",
                        "grant_config": {
                            "description": "Staking grant",
                            "authorization": {
                                "type_url": "/cosmos.authz.v1.GenericAuthorization",
                                "value": "CgREZWxlZ2F0ZQ=="
                            },
                            "optional": false
                        }
                    }
                ],
                "fee_configs": [
                    {
                        "description": "Fee allowance for user1",
                        "allowance": {
                            "type_url": "/cosmos.feegrant.v1.BasicAllowance",
                            "value": "CgQICAI="
                        },
                        "expiration": 1715151235
                    }
                ]
            }
        }
        "#;

        // Deserialize JSON into ExecuteMsg
        let execute_msg: ExecuteMsg = serde_json::from_str(json_msg).unwrap();

        // Act: Call the execute function with the deserialized message
        let result = execute(deps.as_mut(), env.clone(), info.clone(), execute_msg);

        // Assert: Ensure the response is successful
        match result {
            Ok(response) => {
                // Ensure the event is emitted
                assert!(response.events.iter().any(|e| e.ty == "update_configs"));

                // Validate grant configs were saved
                let grants = ["/cosmos.bank.v1.MsgSend", "/cosmos.staking.v1.MsgDelegate"];
                for &grant_key in grants.iter() {
                    let exists = GRANT_CONFIGS.has(deps.as_ref().storage, grant_key.to_string());
                    assert!(exists);

                    let stored_grant = GRANT_CONFIGS
                        .load(deps.as_ref().storage, grant_key.to_string())
                        .unwrap();
                    if grant_key == "/cosmos.bank.v1.MsgSend" {
                        assert_eq!(stored_grant.description, "Bank grant");
                        assert_eq!(
                            stored_grant.authorization,
                            Any {
                                type_url: "/cosmos.authz.v1.GenericAuthorization".to_string(),
                                value: vec![0x0A, 0x04, 0x50, 0x61, 0x79].into(),
                            }
                        );
                        assert!(stored_grant.optional);
                    } else if grant_key == "/cosmos.staking.v1.MsgDelegate" {
                        assert_eq!(stored_grant.description, "Staking grant");
                        assert_eq!(
                            stored_grant.authorization,
                            Any {
                                type_url: "/cosmos.authz.v1.GenericAuthorization".to_string(),
                                value: vec![
                                    0x0A, 0x04, 0x44, 0x65, 0x6C, 0x65, 0x67, 0x61, 0x74, 0x65
                                ]
                                .into(),
                            }
                        );
                        assert!(!stored_grant.optional);
                    }
                }

                // Validate fee configs were saved
                let fee_config = FEE_CONFIG.load(deps.as_ref().storage).unwrap();
                assert_eq!(fee_config.description, "Fee allowance for user1");
                assert_eq!(
                    fee_config.allowance.unwrap(),
                    Any {
                        type_url: "/cosmos.feegrant.v1.BasicAllowance".to_string(),
                        value: b"\x0a\x04\x08\x08\x02".into(),
                    }
                );
                assert_eq!(fee_config.expiration.unwrap(), 1715151235);
            }
            Err(err) => panic!("Test failed with error: {:?}", err),
        }
    }

    #[test]
    fn test_update_configs_with_none_values() {
        // Arrange: Set up environment and dependencies
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = message_info(&Addr::unchecked("admin"), &[]);

        // Mock admin address in storage
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked("admin"))
            .unwrap();

        // Example JSON with `null` for grants and fee_configs
        let json_msg = r#"
        {
            "update_configs": {
                "grants": null,
                "fee_configs": null
            }
        }
        "#;

        // Deserialize JSON into ExecuteMsg
        let execute_msg: ExecuteMsg = serde_json::from_str(json_msg).unwrap();

        // Act: Call the execute function with no grants or fee_configs
        let result = execute(deps.as_mut(), env.clone(), info.clone(), execute_msg);

        // Assert: Ensure the response is successful
        match result {
            Ok(response) => {
                // Ensure the event is emitted
                assert!(response.events.iter().any(|e| e.ty == "update_configs"));

                // Ensure no grants or fees were added
                assert!(GRANT_CONFIGS
                    .keys(
                        deps.as_ref().storage,
                        None,
                        None,
                        cosmwasm_std::Order::Ascending
                    )
                    .next()
                    .is_none());
                assert!(FEE_CONFIG
                    .may_load(deps.as_ref().storage)
                    .unwrap()
                    .is_none());
            }
            Err(err) => panic!("Test failed with error: {:?}", err),
        }
    }

    #[test]
    fn test_update_configs_with_invalid_json() {
        // Arrange: Set up environment and dependencies
        let mut deps = mock_dependencies();
        // Mock admin address in storage
        ADMIN
            .save(deps.as_mut().storage, &Addr::unchecked("admin"))
            .unwrap();

        // Invalid JSON input
        let invalid_json_msg = r#"
        {
            "update_configs": {
                "grants": [
                    { "msg_type_url": "/invalid/url" }
                ]
            }
        }
        "#;

        // Deserialize JSON into ExecuteMsg
        let execute_msg: Result<ExecuteMsg, _> = serde_json::from_str(invalid_json_msg);

        // Assert: Ensure deserialization fails
        assert!(execute_msg.is_err());
    }
}

#[cfg(test)]
pub mod unit_test {
    use core::marker::PhantomData;
    use cosmos_sdk_proto::cosmos::authz::v1beta1::{QueryGrantsRequest, QueryGrantsResponse};
    use cosmos_sdk_proto::prost::Message;
    use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        to_json_vec, Binary, ContractResult, CustomQuery, OwnedDeps, QueryRequest, SystemResult,
    };
    use serde::{Deserialize, Serialize};

    use crate::contract::{execute, instantiate};
    use crate::grant::{Any, FeeConfig, GrantConfig};
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use crate::unit_test::responses::GRANTS_QUERY_RESPONSE_BYTES;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum MyCustomQuery {
        GrantsQuery(QueryGrantsRequest),
    }
    impl CustomQuery for MyCustomQuery {}

    const GRANTER: &str = "granter";
    const GRANTEE: &str = "grantee";

    #[test]
    fn test_query_grants_binary_response() {
        let querier = MockQuerier::<MyCustomQuery>::with_custom_handler(
            MockQuerier::<MyCustomQuery>::new(&[]),
            custom_querier,
        );

        let mut owned = OwnedDeps::<_, _, _, MyCustomQuery> {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier,
            custom_query_type: PhantomData,
        };
        let deps = owned.as_mut();

        let grant_config = GrantConfig {
            description: "test fee grant".to_string(),
            authorization: Any {
                type_url: "/cosmos.bank.v1beta1.SendAuthorization".to_string(),
                value: Binary::from_base64("ChAKBXV4aW9uEgcxMDAwMDAw").unwrap(),
            },
            optional: false,
        };

        let msg_type_url = "msg_type_url".to_string();
        let query_msg = QueryGrantsRequest {
            granter: GRANTER.to_string(),
            grantee: GRANTEE.to_string(),
            msg_type_url: msg_type_url.clone(),
            pagination: None,
        };

        let custom_query = QueryRequest::Custom(MyCustomQuery::GrantsQuery(query_msg));
        let query_res = deps.querier.raw_query(&to_json_vec(&custom_query).unwrap());
        match query_res {
            SystemResult::Ok(ContractResult::Ok(res)) => {
                assert_eq!(res, GRANTS_QUERY_RESPONSE_BYTES);
                let res = QueryGrantsResponse::decode::<&[u8]>(res.as_slice());
                match res {
                    Ok(res) => {
                        assert_eq!(res.grants.len(), 1);
                        let authorization = res.grants[0].authorization.as_ref().unwrap();

                        assert_eq!(*authorization, grant_config.authorization.into());
                    }
                    Err(err) => {
                        panic!("Error in deserializing query result: {:?}", err);
                    }
                }
            }
            SystemResult::Ok(ContractResult::Err(err)) => panic!("{:?}", err),
            SystemResult::Err(err) => panic!("{:?}", err),
        }
    }
    #[test]
    fn test_query_grants_request() {
        let querier = MockQuerier::<MyCustomQuery>::with_custom_handler(
            MockQuerier::<MyCustomQuery>::new(&[]),
            custom_querier,
        );

        let mut owned = OwnedDeps::<_, _, _, MyCustomQuery> {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier,
            custom_query_type: PhantomData,
        };
        let mut deps = owned.as_mut();
        let env = mock_env();

        let info = mock_info(&GRANTER, &[]);

        let instantiate_msg = InstantiateMsg {
            admin: None,
            type_urls: vec!["/cosmos.bank.v1beta1.MsgSend".to_string()],
            grant_configs: vec![GrantConfig {
                description: "test fee grant".to_string(),
                authorization: Any {
                    type_url: "/cosmos.bank.v1beta1.SendAuthorization".to_string(),
                    value: Binary::from_base64("ChAKBXV4aW9uEgcxMDAwMDAw").unwrap(),
                },
                optional: false,
            }],
            fee_config: FeeConfig {
                description: "test fee grant".to_string(),
                allowance: Some(Any {
                    type_url: "/cosmos.feegrant.v1beta1.BasicAllowance".to_string(),
                    value: Binary::from_base64("EgsI2b/mtAYQ4KOeNA==").unwrap(),
                }),
                expiration: Some(18000),
            },
        };

        instantiate(
            deps.branch().into_empty(),
            env.clone(),
            info.clone(),
            instantiate_msg,
        )
        .expect("instantiate successful");

        let execute_msg = ExecuteMsg::DeployFeeGrant {
            authz_granter: deps.api.addr_validate(&GRANTER).unwrap(),
            authz_grantee: deps.api.addr_validate(&GRANTEE).unwrap(),
        };

        let execute_res = execute(
            deps.branch().into_empty(),
            env.clone(),
            info.clone(),
            execute_msg,
        );

        match execute_res {
            Ok(res) => assert_eq!(res.messages.len(), 1),
            Err(err) => panic!("{:?}", err),
        }
    }

    /// this function simulates a stargate request to the querier
    fn custom_querier(request: &MyCustomQuery) -> SystemResult<ContractResult<Binary>> {
        match request {
            MyCustomQuery::GrantsQuery(_) => {
                return cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                    Binary::from(GRANTS_QUERY_RESPONSE_BYTES),
                ));
            }
        };
    }
}

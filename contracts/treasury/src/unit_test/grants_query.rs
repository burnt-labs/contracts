#[cfg(test)]
pub mod unit_test {
    use core::marker::PhantomData;
    use cosmos_sdk_proto::cosmos::authz::v1beta1::QueryGrantsRequest;
    use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        to_json_binary, Binary, ContractResult, CustomQuery, OwnedDeps, QueryRequest, SystemResult,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};

    use crate::unit_test::responses::GRANTS_QUERY_RESPONSE;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum MyCustomQuery {
        GrantsQuery(QueryGrantsRequest),
    }
    impl CustomQuery for MyCustomQuery {}

    #[test]
    fn test_query_grants_request() {
        let querier = MockQuerier::<MyCustomQuery>::with_custom_handler(
            MockQuerier::<MyCustomQuery>::new(&[]),
            custom_querier,
        );

        let owned = OwnedDeps::<_, _, _, MyCustomQuery> {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier,
            custom_query_type: PhantomData,
        };
        let deps = owned.as_ref();

        let granter = "granter".to_string();
        let grantee = "grantee".to_string();
        let msg_type_url = "msg_type_url".to_string();
        let query_msg = QueryGrantsRequest {
            granter: granter.clone(),
            grantee: grantee.clone(),
            msg_type_url: msg_type_url.clone(),
            pagination: None,
        };
        let custom_query = QueryRequest::Custom(MyCustomQuery::GrantsQuery(query_msg));
        let query_res = deps.querier.query::<Value>(&custom_query);
        match query_res {
            Ok(res) => {
                assert_eq!(res, json!(GRANTS_QUERY_RESPONSE));
            }
            Err(err) => {
                panic!("Error in querying grants: {:?}", err);
            }
        }
    }

    /// this function simulates a stargate request to the querier
    fn custom_querier(request: &MyCustomQuery) -> SystemResult<ContractResult<Binary>> {
        match request {
            MyCustomQuery::GrantsQuery(_) => {
                let grants = json!(GRANTS_QUERY_RESPONSE);
                match to_json_binary(&grants) {
                    Ok(res) => {
                        return cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                            res,
                        ));
                    }
                    Err(err) => {
                        panic!("Error in serializing query result: {:?}", err);
                    }
                }
            }
        };
    }
}

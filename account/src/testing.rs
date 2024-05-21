use std::marker::PhantomData;

use crate::msg::{SpecialQuery, SpecialResponse};

use crate::proto::{QueryDomainInfoResponse, XionCustomQuery};
use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{to_json_binary, Binary, Coin, ContractResult, OwnedDeps, SystemResult};

pub fn mock_dependencies_with_custom_querier(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, MockQuerier<XionCustomQuery>, XionCustomQuery> {
    let custom_querier: MockQuerier<XionCustomQuery> =
        MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)])
            .with_custom_handler(|query| SystemResult::Ok(custom_query_execute(query)));
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub fn custom_query_execute(query: &XionCustomQuery) -> ContractResult<Binary> {
    let msg = match query {
        XionCustomQuery::DomainInfo { .. } => QueryDomainInfoResponse{ domain: "gmail.com".to_string(), info: "".to_string() },
        _ => { return Err()}
    };
    to_json_binary(&SpecialResponse { msg }).into()
}

use cosmos_sdk_proto::cosmos::authz::v1beta1::QueryGrantsRequest;
use cosmwasm_std::CustomQuery;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum XionCustomQuery {
    Grants(QueryGrantsRequest),
}

impl CustomQuery for XionCustomQuery {}

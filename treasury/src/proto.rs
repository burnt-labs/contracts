use cosmwasm_std::CustomQuery;
use osmosis_std_derive::CosmwasmExt;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    PartialEq,
    Eq,
    ::prost::Message,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    CosmwasmExt,
)]
#[proto_message(type_url = "/xion.v1.Query/WebAuthNVerifyAuthenticate")]
#[proto_query(path = "/xion.v1.Query/WebAuthNVerifyAuthenticate", response_type = QueryGrantsResponse)]
pub struct QueryGrantsRequest {
    #[prost(string, tag = "1", optional)]
    pub granter: Option<String>,
    #[prost(string, tag = "2", optional)]
    pub grantee: Option<String>,
    #[prost(string, tag = "3", optional)]
    pub msg_type_url: Option<String>,
    #[prost(string, tag = "4", optional)]
    pub pagination: Option<String>,
}

// Redefining these structs because the cosmos-sdk-proto crate is structs do not implement serde traits Serialize and Deserialize
#[derive(Clone, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct QueryGrantsResponse {
    #[prost(message, repeated, tag = "1")]
    pub grants: Vec<Grant>,
    #[prost(message, tag = "2", optional)]
    pub pagination: Option<PageResponse>,
}

#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct Grant {
    #[prost(message, optional, tag = "1")]
    pub authorization: Option<Any>,
    /// time when the grant will expire and will be pruned. If null, then the grant
    /// doesn't have a time expiration (other conditions  in `authorization`
    /// may apply to invalidate the grant)
    #[prost(message, optional, tag = "2")]
    pub expiration: Option<Timestamp>,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct Any {
    #[prost(string, tag = "1")]
    pub type_url: ::prost::alloc::string::String,
    /// Must be a valid serialized protocol buffer of the above specified type.
    #[prost(bytes = "vec", tag = "2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct Timestamp {
    /// Represents seconds of UTC time since Unix epoch
    /// 1970-01-01T00:00:00Z. Must be from 0001-01-01T00:00:00Z to
    /// 9999-12-31T23:59:59Z inclusive.
    #[prost(int64, tag = "1")]
    pub seconds: i64,
    /// Non-negative fractions of a second at nanosecond resolution. Negative
    /// second values with fractions must still have non-negative nanos values
    /// that count forward in time. Must be from 0 to 999,999,999
    /// inclusive.
    #[prost(int32, tag = "2")]
    pub nanos: i32,
}

#[derive(Clone, PartialEq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct PageResponse {
    /// next_key is the key to be passed to PageRequest.key to
    /// query the next page most efficiently. It will be empty if
    /// there are no more results.
    #[prost(bytes = "vec", tag = "1")]
    pub next_key: ::prost::alloc::vec::Vec<u8>,
    /// total is total number of results available if PageRequest.count_total
    /// was set, its value is undefined otherwise
    #[prost(uint64, tag = "2")]
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum XionCustomQuery {
    Grants(QueryGrantsRequest),
}

impl CustomQuery for XionCustomQuery {}

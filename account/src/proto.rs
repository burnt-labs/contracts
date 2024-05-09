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
#[proto_message(type_url = "/xion.v1.Query/WebAuthNVerifyRegister")]
#[proto_query(path = "/xion.v1.Query/WebAuthNVerifyRegister", response_type = QueryWebAuthNVerifyRegisterResponse)]
pub struct QueryWebAuthNVerifyRegisterRequest {
    #[prost(string, tag = "1")]
    pub addr: String,
    #[prost(string, tag = "2")]
    pub challenge: String,
    #[prost(string, tag = "3")]
    pub rp: String,
    #[prost(bytes, tag = "4")]
    pub data: Vec<u8>,
}

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
#[proto_query(path = "/xion.v1.Query/WebAuthNVerifyAuthenticate", response_type = QueryWebAuthNVerifyAuthenticateResponse)]
pub struct QueryWebAuthNVerifyAuthenticateRequest {
    #[prost(string, tag = "1")]
    pub addr: String,
    #[prost(string, tag = "2")]
    pub challenge: String,
    #[prost(string, tag = "3")]
    pub rp: String,
    #[prost(bytes, tag = "4")]
    pub credential: Vec<u8>,
    #[prost(bytes, tag = "5")]
    pub data: Vec<u8>,
}

// We define the response as a prost message to be able to decode the protobuf data.
#[derive(Clone, PartialEq, Eq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct QueryWebAuthNVerifyRegisterResponse {
    #[prost(bytes, tag = "1")]
    pub credential: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct QueryWebAuthNVerifyAuthenticateResponse {}

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
#[proto_message(type_url = "/xion.jwk.v1.Query/ValidateJWT")]
#[proto_query(path = "/xion.jwk.v1.Query/ValidateJWT", response_type = QueryValidateJWTResponse)]
pub struct QueryValidateJWTRequest {
    #[prost(string, tag = "1")]
    pub aud: String,
    #[prost(string, tag = "2")]
    pub sub: String,
    #[prost(string, tag = "3")]
    pub sig_bytes: String,
    // #[prost(string, tag = "4")]
    // pub tx_hash: String,
}

#[derive(Clone, PartialEq, Eq, ::prost::Message, serde::Serialize, serde::Deserialize)]
pub struct QueryValidateJWTResponse {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum XionCustomQuery {
    Verify(QueryWebAuthNVerifyRegisterRequest),
    Authenticate(QueryWebAuthNVerifyAuthenticateRequest),
    JWTValidate(QueryValidateJWTRequest),
}
impl CustomQuery for XionCustomQuery {}

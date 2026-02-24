use cosmwasm_schema::{cw_serde, QueryResponses};
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    /// The audience identifier registered in xion's JWK module
    /// for the Play Integrity verification key (EC P-256 JWK).
    pub aud: String,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Verify a Play Integrity Classic JWS token and return the verdict.
    /// The relayer decrypts the outer JWE off-chain and submits the inner JWS.
    ///
    /// Delegates signature verification to xion's JWK module via the
    /// VerifyJWS gRPC query, then parses the verified payload.
    #[returns(VerifyResponse)]
    Verify { compact_jws: String },

    #[returns(String)]
    GetAud {},
}

#[cw_serde]
pub struct VerifyResponse {
    pub verdict: IntegrityVerdict,
}

/// Play Integrity verdict payload (Google API uses camelCase).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityVerdict {
    pub request_details: RequestDetails,
    pub app_integrity: AppIntegrity,
    pub device_integrity: DeviceIntegrity,
    pub account_details: AccountDetails,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RequestDetails {
    pub request_package_name: String,
    pub nonce: String,
    pub timestamp_millis: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppIntegrity {
    pub app_recognition_verdict: String,
    pub package_name: Option<String>,
    #[serde(default)]
    pub certificate_sha256_digest: Option<Vec<String>>,
    pub version_code: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIntegrity {
    pub device_recognition_verdict: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetails {
    pub app_licensing_verdict: String,
}

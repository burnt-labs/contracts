use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub struct ExecuteMsg {}

#[cw_serde]
pub struct VerifyAttestation {
    pub app_id: String,
    pub key_id: String,
    pub challenge: Binary,
    pub cbor_data: Binary,
    pub dev_env: Option<bool>,
}

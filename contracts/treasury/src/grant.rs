pub mod allowance;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use serde_json::Value;

#[cw_serde]
pub struct GrantConfig {
    description: String,
    pub authorization: Value,
    pub optional: bool,
}

#[cw_serde]
pub struct FeeConfig {
    description: String,
    pub allowance: Option<Any>,
    pub expiration: Option<u32>,
}

#[cw_serde]
pub struct Any {
    pub type_url: String,
    pub value: Binary,
}

impl From<cosmos_sdk_proto::Any> for Any {
    fn from(value: cosmos_sdk_proto::Any) -> Self {
        Any {
            type_url: value.type_url,
            value: Binary::from(value.value),
        }
    }
}

impl From<Any> for cosmos_sdk_proto::Any {
    fn from(value: Any) -> Self {
        cosmos_sdk_proto::Any {
            type_url: value.type_url,
            value: value.value.to_vec(),
        }
    }
}

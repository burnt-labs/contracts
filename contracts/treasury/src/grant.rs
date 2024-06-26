pub mod allowance;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use prost::bytes::Bytes;
use serde_json::Value;

#[cw_serde]
pub struct GrantConfig {
    description: String,
    pub authorization: Value,
    pub allowance: Option<Any>,
    pub max_duration: Option<u32>,
}

#[cw_serde]
pub struct Any {
    pub type_url: String,
    pub value: Binary,
}

impl From<pbjson_types::Any> for Any {
    fn from(value: pbjson_types::Any) -> Self {
        Any {
            type_url: value.type_url,
            value: Binary::from(value.value.to_vec()),
        }
    }
}

impl From<Any> for pbjson_types::Any {
    fn from(value: Any) -> Self {
        pbjson_types::Any {
            type_url: value.type_url,
            value: Bytes::copy_from_slice(value.value.as_slice()),
        }
    }
}

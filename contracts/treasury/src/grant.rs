pub mod allowance;

use cosmos_sdk_proto::prost::bytes::Bytes;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

#[cw_serde]
pub struct GrantConfig {
    description: String,
    pub authorization: Any,
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
            value: Bytes::from(value.value.to_vec()),
        }
    }
}

pub mod allowance;

use cosmos_sdk_proto::{prost::Name, traits::MessageExt};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;

use crate::error::ContractResult;

#[cw_serde]
pub struct GrantConfig {
    description: String,
    pub authorization: Any,
    pub optional: bool,
}

#[cw_serde]

pub enum AuthorizationData {
    Any(Any),
    ExecuteOnAccount(AuthorizationOnAccount),
}

#[cw_serde]

pub struct AuthorizationOnAccount {
    pub limit: Option<Any>,
    pub filter: Option<Any>,
}

#[cw_serde]

pub struct GrantConfigStorage {
    pub description: String,
    pub authorization: AuthorizationData,
    pub optional: bool,
}

impl GrantConfigStorage {
    pub fn try_into_grant_config(self, address: String) -> ContractResult<GrantConfig> {
        Ok(GrantConfig {
            description: self.description,
            authorization: match self.authorization {
                AuthorizationData::Any(any) => any,
                AuthorizationData::ExecuteOnAccount(auth) => {
                    // Handle the case where authorization is on the account itself
                    Any {
                        type_url: cosmos_sdk_proto::cosmwasm::wasm::v1::ContractExecutionAuthorization::full_name(),
                        value:
                            cosmos_sdk_proto::cosmwasm::wasm::v1::ContractExecutionAuthorization {
                                grants: vec![cosmos_sdk_proto::cosmwasm::wasm::v1::ContractGrant {
                                    contract: address,
                                    limit: auth.limit.map(Into::into),
                                    filter:auth.filter.map(Into::into),
                                }],
                            }
                            .to_bytes()?
                            .into(),
                    }
                }
            },
            optional: self.optional,
        })
    }
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

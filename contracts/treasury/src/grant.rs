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

impl AuthorizationData {
    pub fn try_into_any(self, address: String) -> ContractResult<Any> {
        Ok(match self {
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
        })
    }
}

impl GrantConfigStorage {
    pub fn try_into_grant_config(self, address: String) -> ContractResult<GrantConfig> {
        Ok(GrantConfig {
            description: self.description,
            authorization: self.authorization.try_into_any(address)?,
            optional: self.optional,
        })
    }
}

#[cw_serde]
pub struct FeeConfigStorage {
    description: String,
    pub allowance: Option<AllowanceData>,
    pub expiration: Option<u32>,
}

#[cw_serde]
pub struct FeeConfig {
    description: String,
    pub allowance: Option<Any>,
    pub expiration: Option<u32>,
}

#[cw_serde]
pub enum AllowanceData {
    Any(Any),
    AllowanceOnAccount(AllowanceOnAccount),
}
#[cw_serde]
pub struct AllowanceOnAccount {
    pub allowance: Option<Any>,
}

impl FeeConfigStorage {
    pub fn try_into_fee_config(self, address: String) -> ContractResult<FeeConfig> {
        Ok(FeeConfig {
            description: self.description,
            allowance: match self.allowance {
                None => None,
                Some(allowance) => Some(allowance.try_into_any(address)?),
            },
            expiration: self.expiration,
        })
    }
}

impl AllowanceData {
    pub fn try_into_any(self, address: String) -> ContractResult<Any> {
        Ok(match self {
            AllowanceData::Any(any) => any,
            AllowanceData::AllowanceOnAccount(allowance) => {
                // Handle the case where authorization is on the account itself
                Any {
                    type_url: cosmos_sdk_proto::cosmwasm::wasm::v1::ContractExecutionAuthorization::full_name(),
                    value:
                        cosmos_sdk_proto::xion::v1::ContractsAllowance {
                            allowance: allowance.allowance.map(Into::into),
                            contract_addresses: vec![address]
                        }
                        .to_bytes()?
                        .into(),
                }
            }
        })
    }
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

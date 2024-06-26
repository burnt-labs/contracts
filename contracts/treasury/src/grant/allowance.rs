use crate::error::ContractError::{self, AllowanceUnset, InvalidAllowanceType};
use crate::error::ContractResult;
use crate::grant::Any;
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::{
    AllowedMsgAllowance, BasicAllowance, PeriodicAllowance,
};
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::xion::v1::{AuthzAllowance, ContractsAllowance};
use cosmwasm_std::Addr;
use pbjson_types::Timestamp;

pub fn format_allowance(
    allowance_any: Any,
    _granter: Addr,
    grantee: Addr,
    expiration: Option<Timestamp>,
) -> ContractResult<Any> {
    let formatted_allowance: Any = match allowance_any.type_url.as_str() {
        "/cosmos.feegrant.v1beta1.BasicAllowance" => match expiration.clone() {
            None => allowance_any,
            Some(_) => {
                let mut allowance = BasicAllowance::decode::<&[u8]>(allowance_any.value.as_slice())
                    .map_err(|err| {
                        ContractError::Std(cosmwasm_std::StdError::ParseErr {
                            target_type: "Basic Allowance".to_string(),
                            msg: err.to_string(),
                        })
                    })?;
                allowance.expiration = expiration;
                let allowance_bz = match allowance.to_bytes() {
                    Ok(bz) => bz,
                    Err(_) => {
                        return Err(ContractError::Std(cosmwasm_std::StdError::SerializeErr {
                            source_type: String::from("BasicAllowance"),
                            msg: "unable to serialize basic allowance".to_string(),
                        }))
                    }
                };
                Any {
                    type_url: allowance_any.type_url,
                    value: allowance_bz.into(),
                }
            }
        },

        "/cosmos.feegrant.v1beta1.PeriodicAllowance" => match expiration.clone() {
            None => allowance_any,
            Some(_) => {
                let mut allowance = PeriodicAllowance::decode::<&[u8]>(
                    allowance_any.value.as_slice(),
                )
                .map_err(|err| {
                    ContractError::Std(cosmwasm_std::StdError::ParseErr {
                        target_type: "Periodic Allowance".to_string(),
                        msg: err.to_string(),
                    })
                })?;
                let mut inner_basic = allowance.basic.clone().ok_or(AllowanceUnset)?;
                inner_basic.expiration = expiration;
                allowance.basic = Some(inner_basic);
                let allowance_bz = match allowance.to_bytes() {
                    Ok(bz) => bz,
                    Err(_) => {
                        return Err(ContractError::Std(cosmwasm_std::StdError::SerializeErr {
                            source_type: String::from("PeriodicAllowance"),
                            msg: "unable to serialize periodic allowance".to_string(),
                        }))
                    }
                };
                Any {
                    type_url: allowance_any.type_url,
                    value: allowance_bz.into(),
                }
            }
        },

        "/cosmos.feegrant.v1beta1.AllowedMsgAllowance" => {
            let mut allowance = AllowedMsgAllowance::decode::<&[u8]>(
                allowance_any.value.as_slice(),
            )
            .map_err(|err| {
                ContractError::Std(cosmwasm_std::StdError::ParseErr {
                    target_type: "Allowed Msg Allowance".to_string(),
                    msg: err.to_string(),
                })
            })?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                _granter,
                grantee,
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            let allowance_bz = match allowance.to_bytes() {
                Ok(bz) => bz,
                Err(_) => {
                    return Err(ContractError::Std(cosmwasm_std::StdError::SerializeErr {
                        source_type: String::from("AllowedMsgAllowance"),
                        msg: "unable to serialize allowed msg allowance".to_string(),
                    }))
                }
            };
            Any {
                type_url: allowance_any.type_url,
                value: allowance_bz.into(),
            }
        }

        "/xion.v1.AuthzAllowance" => {
            let mut allowance = AuthzAllowance::decode::<&[u8]>(allowance_any.value.as_slice())
                .map_err(|err| {
                    ContractError::Std(cosmwasm_std::StdError::ParseErr {
                        target_type: "Authz Allowance".to_string(),
                        msg: err.to_string(),
                    })
                })?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                _granter,
                grantee.clone(),
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            allowance.authz_grantee = grantee.into_string();
            let allowance_bz = match allowance.to_bytes() {
                Ok(bz) => bz,
                Err(_) => {
                    return Err(ContractError::Std(cosmwasm_std::StdError::SerializeErr {
                        source_type: String::from("AuthzAllowance"),
                        msg: "unable to serialize authz allowance".to_string(),
                    }))
                }
            };
            Any {
                type_url: allowance_any.type_url,
                value: allowance_bz.into(),
            }
        }

        "/xion.v1.ContractsAllowance" => {
            let mut allowance = ContractsAllowance::decode::<&[u8]>(allowance_any.value.as_slice())
                .map_err(|err| {
                    ContractError::Std(cosmwasm_std::StdError::ParseErr {
                        target_type: "Contract Allowance".to_string(),
                        msg: err.to_string(),
                    })
                })?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                _granter,
                grantee.clone(),
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            let allowance_bz = match allowance.to_bytes() {
                Ok(bz) => bz,
                Err(_) => {
                    return Err(ContractError::Std(cosmwasm_std::StdError::SerializeErr {
                        source_type: String::from("ContractAllowance"),
                        msg: "unable to serialize contract allowance".to_string(),
                    }))
                }
            };
            Any {
                type_url: allowance_any.type_url,
                value: allowance_bz.into(),
            }
        }
        _ => {
            return Err(InvalidAllowanceType {
                msg_type_url: allowance_any.type_url,
            })
        }
    };

    Ok(formatted_allowance)
}

use crate::error::ContractError::{self, AllowanceUnset, InvalidAllowanceType};
use crate::error::ContractResult;
use crate::grant::Any;
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::{
    AllowedMsgAllowance, BasicAllowance, PeriodicAllowance,
};
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::xion::v1::{AuthzAllowance, ContractsAllowance, MultiAnyAllowance};
use cosmos_sdk_proto::Timestamp;
use cosmwasm_std::Addr;

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
                let mut allowance = BasicAllowance::decode(allowance_any.value.as_slice())?;
                allowance.expiration = expiration;
                let allowance_bz = allowance.to_bytes()?;
                Any {
                    type_url: allowance_any.type_url,
                    value: allowance_bz.into(),
                }
            }
        },

        "/cosmos.feegrant.v1beta1.PeriodicAllowance" => match expiration.clone() {
            None => allowance_any,
            Some(_) => {
                let mut allowance = PeriodicAllowance::decode(allowance_any.value.as_slice())?;
                let mut inner_basic = allowance.basic.clone().ok_or(AllowanceUnset)?;
                inner_basic.expiration = expiration;
                allowance.basic = Some(inner_basic);
                let allowance_bz = allowance.to_bytes()?;
                Any {
                    type_url: allowance_any.type_url,
                    value: allowance_bz.into(),
                }
            }
        },

        "/cosmos.feegrant.v1beta1.AllowedMsgAllowance" => {
            let mut allowance = AllowedMsgAllowance::decode(allowance_any.value.as_slice())?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                _granter,
                grantee,
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            let allowance_bz = allowance.to_bytes()?;
            Any {
                type_url: allowance_any.type_url,
                value: allowance_bz.into(),
            }
        }

        "/xion.v1.AuthzAllowance" => {
            let mut allowance = AuthzAllowance::decode(allowance_any.value.as_slice())?;
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
            let mut allowance = ContractsAllowance::decode(allowance_any.value.as_slice())?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                _granter,
                grantee.clone(),
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            let allowance_bz = allowance.to_bytes()?;
            Any {
                type_url: allowance_any.type_url,
                value: allowance_bz.into(),
            }
        }
        "/xion.v1.MultiAnyAllowance" => {
            let mut allowance = MultiAnyAllowance::decode(allowance_any.value.as_slice())?;

            for inner_allowance in allowance.allowances.iter_mut() {
                *inner_allowance = format_allowance(
                    inner_allowance.clone().into(),
                    _granter.clone(),
                    grantee.clone(),
                    expiration.clone(),
                )?
                .into();
            }

            let allowance_bz = allowance.to_bytes()?;
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

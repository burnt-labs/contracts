use crate::error::ContractError::{AllowanceUnset, InvalidAllowanceType};
use crate::error::ContractResult;
use crate::grant::Any;
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::{
    AllowedMsgAllowance, BasicAllowance, PeriodicAllowance,
};
use cosmos_sdk_proto::xion::v1::{AuthzAllowance, ContractsAllowance};
use cosmwasm_std::Addr;
use pbjson_types::Timestamp;

pub fn format_allowance(
    allowance_any: Any,
    granter: Addr,
    grantee: Addr,
    expiration: Option<Timestamp>,
) -> ContractResult<Any> {
    let formatted_allowance: Any = match allowance_any.msg_type_url.as_str() {
        "/cosmos.feegrant.v1beta1.BasicAllowance" => match expiration.clone() {
            None => allowance_any,
            Some(_) => {
                let mut allowance: BasicAllowance =
                    cosmwasm_std::from_binary(&allowance_any.value)?;
                allowance.expiration = expiration;
                let allowance_bz = cosmwasm_std::to_binary(&allowance)?;
                Any {
                    msg_type_url: allowance_any.msg_type_url,
                    value: allowance_bz,
                }
            }
        },

        "/cosmos.feegrant.v1beta1.PeriodicAllowance" => match expiration.clone() {
            None => allowance_any,
            Some(_) => {
                let mut allowance: PeriodicAllowance =
                    cosmwasm_std::from_binary(&allowance_any.value)?;
                allowance.basic.ok_or(AllowanceUnset)?.expiration = expiration;
                let allowance_bz = cosmwasm_std::to_binary(&allowance)?;
                Any {
                    msg_type_url: allowance_any.msg_type_url,
                    value: allowance_bz,
                }
            }
        },

        "/cosmos.feegrant.v1beta1.AllowedMsgAllowance" => {
            let mut allowance: AllowedMsgAllowance =
                cosmwasm_std::from_binary(&allowance_any.value)?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                granter,
                grantee,
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            let allowance_bz = cosmwasm_std::to_binary(&allowance)?;
            Any {
                msg_type_url: allowance_any.msg_type_url,
                value: allowance_bz,
            }
        }

        "/xion.v1.AuthzAllowance" => {
            let mut allowance: AuthzAllowance = cosmwasm_std::from_binary(&allowance_any.value)?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                granter,
                grantee.clone(),
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            allowance.authz_grantee = grantee.into_string();
            let allowance_bz = cosmwasm_std::to_binary(&allowance)?;
            Any {
                msg_type_url: allowance_any.msg_type_url,
                value: allowance_bz,
            }
        }

        "/xion.v1.ContractsAllowance" => {
            let mut allowance: ContractsAllowance =
                cosmwasm_std::from_binary(&allowance_any.value)?;
            let inner_allowance = format_allowance(
                allowance.allowance.ok_or(AllowanceUnset)?.into(),
                granter,
                grantee.clone(),
                expiration,
            )?;
            allowance.allowance = Some(inner_allowance.into());
            let allowance_bz = cosmwasm_std::to_binary(&allowance)?;
            Any {
                msg_type_url: allowance_any.msg_type_url,
                value: allowance_bz,
            }
        }
        _ => {
            return Err(InvalidAllowanceType {
                msg_type_url: allowance_any.msg_type_url,
            })
        }
    };

    Ok(formatted_allowance)
}

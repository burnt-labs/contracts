use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Empty};

use crate::ContractError;
use crate::ContractError::InvalidMsgType;
use cw721::{
    msg::{Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721MigrateMsg, Cw721QueryMsg},
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    EmptyOptionalNftExtension, EmptyOptionalNftExtensionMsg,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub proxy_addr: Addr,

    pub inner_msg: Cw721InstantiateMsg<DefaultOptionalCollectionExtensionMsg>,
}

pub type ExecuteMsg =
    Cw721ExecuteMsg<EmptyOptionalNftExtensionMsg, DefaultOptionalCollectionExtensionMsg, ProxyMsg>;
// pub type InstantiateMsg = Cw721InstantiateMsg<DefaultOptionalCollectionExtensionMsg>;
pub type MigrateMsg = Cw721MigrateMsg;
pub type QueryMsg =
    Cw721QueryMsg<EmptyOptionalNftExtension, DefaultOptionalCollectionExtension, Empty>;

pub type InnerExecuteMsg =
    Cw721ExecuteMsg<EmptyOptionalNftExtensionMsg, DefaultOptionalCollectionExtensionMsg, Empty>;

#[cw_serde]
pub struct ProxyMsg {
    pub sender: Addr,
    pub msg: InnerExecuteMsg,
}

pub fn get_inner(msg: ExecuteMsg) -> Result<InnerExecuteMsg, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwnership(_0) => Ok(InnerExecuteMsg::UpdateCreatorOwnership(_0)),
        ExecuteMsg::UpdateMinterOwnership(_0) => Ok(InnerExecuteMsg::UpdateCreatorOwnership(_0)),
        ExecuteMsg::UpdateCreatorOwnership(_0) => Ok(InnerExecuteMsg::UpdateCreatorOwnership(_0)),
        ExecuteMsg::UpdateCollectionInfo { collection_info } => {
            Ok(InnerExecuteMsg::UpdateCollectionInfo { collection_info })
        }
        ExecuteMsg::TransferNft {
            recipient,
            token_id,
        } => Ok(InnerExecuteMsg::TransferNft {
            recipient,
            token_id,
        }),
        ExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        } => Ok(InnerExecuteMsg::SendNft {
            contract,
            token_id,
            msg,
        }),
        ExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        } => Ok(InnerExecuteMsg::Approve {
            spender,
            token_id,
            expires,
        }),
        ExecuteMsg::Revoke { spender, token_id } => {
            Ok(InnerExecuteMsg::Revoke { spender, token_id })
        }
        ExecuteMsg::ApproveAll { operator, expires } => {
            Ok(InnerExecuteMsg::ApproveAll { operator, expires })
        }
        ExecuteMsg::RevokeAll { operator } => Ok(InnerExecuteMsg::RevokeAll { operator }),
        ExecuteMsg::Mint {
            token_id,
            owner,
            token_uri,
            extension,
        } => Ok(InnerExecuteMsg::Mint {
            token_id,
            owner,
            token_uri,
            extension,
        }),
        ExecuteMsg::Burn { token_id } => Ok(InnerExecuteMsg::Burn { token_id }),
        ExecuteMsg::UpdateExtension { .. } => Err(InvalidMsgType), // cannot convert a proxy msg into an inner msg
        ExecuteMsg::UpdateNftInfo {
            token_id,
            token_uri,
            extension,
        } => Ok(InnerExecuteMsg::UpdateNftInfo {
            token_id,
            token_uri,
            extension,
        }),
        ExecuteMsg::SetWithdrawAddress { address } => {
            Ok(InnerExecuteMsg::SetWithdrawAddress { address })
        }
        ExecuteMsg::RemoveWithdrawAddress {} => Ok(InnerExecuteMsg::RemoveWithdrawAddress {}),
        ExecuteMsg::WithdrawFunds { amount } => Ok(InnerExecuteMsg::WithdrawFunds { amount }),
    }
}

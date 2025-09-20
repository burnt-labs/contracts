use crate::{
    execute::list,
    msg::{XionAssetCollectionMetadataMsg, XionAssetExtensionExecuteMsg},
    state::{XionAssetCollectionMetadata},
};
use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo, Response};
use cw721::{DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, traits::Cw721Execute};

pub struct AssetContract {}
impl Default for AssetContract {
    fn default() -> Self {
        AssetContract {}
    }
}

impl
    Cw721Execute<
        DefaultOptionalNftExtension,
        DefaultOptionalNftExtensionMsg,
        XionAssetCollectionMetadata,
        XionAssetCollectionMetadataMsg,
        XionAssetExtensionExecuteMsg,
        Empty,
    > for AssetContract
{
    fn execute_extension(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: XionAssetExtensionExecuteMsg,
    ) -> Result<Response<Empty>, cw721::error::Cw721ContractError> {
        match msg {
            XionAssetExtensionExecuteMsg::List { id, price, reserve } => {
                Ok(list::<DefaultOptionalNftExtension, Empty>(
                    deps,
                    env,
                    info,
                    id,
                    price,
                    reserve,
                )?)
            }
            XionAssetExtensionExecuteMsg::Reserve { id, until } => {
                todo!()
            }
            XionAssetExtensionExecuteMsg::Delist { id } => todo!(),
            XionAssetExtensionExecuteMsg::Buy { id, recipient } => {
                todo!()
            }
        }
    }
}

use cosmwasm_std::Empty;
use cw721::{traits::Cw721Execute, DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg};

use crate::{contract::AssetContract, msg::{XionAssetCollectionMetadataMsg, XionAssetExtensionExecuteMsg}, state::XionAssetCollectionMetadata, traits::XionAssetExecuteExtension};


impl Default for AssetContract {
    fn default() -> Self {
        AssetContract {}
    }
}

impl XionAssetExecuteExtension<Empty> for AssetContract {}

impl Cw721Execute<DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, XionAssetCollectionMetadata, XionAssetCollectionMetadataMsg, XionAssetExtensionExecuteMsg, Empty> for AssetContract {
    fn execute_extension(
            &self,
            deps: cosmwasm_std::DepsMut,
            env: &cosmwasm_std::Env,
            info: &cosmwasm_std::MessageInfo,
            msg: XionAssetExtensionExecuteMsg,
        ) -> Result<cosmwasm_std::Response<Empty>, cw721::error::Cw721ContractError> {
            match msg {
                XionAssetExtensionExecuteMsg::List { id, price } => {
                    Ok(self.list(deps, env, info, id, price)?)
                }
                XionAssetExtensionExecuteMsg::FreezeListing { id} => {
                   Ok(self.freeze_listing(deps, env, info, id)?)
                }
                XionAssetExtensionExecuteMsg::Delist { id} => {
                    Ok(self.delist(deps, env, info, id)?)
                }
                XionAssetExtensionExecuteMsg::Buy { id, recipient } => {
                    Ok(self.buy(deps, env, info, id, recipient)?)
                }
            }
    }
}
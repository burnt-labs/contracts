// Default implementation of the xion asset standard showing how to set up a contract
// to use the default trait XionAssetExecuteExtension
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw721::traits::Cw721Execute;

use crate::{error::ContractResult, msg::{InstantiateMsg, XionAssetCollectionMetadataMsg}, CONTRACT_NAME, CONTRACT_VERSION};

pub struct AssetContract {}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg<XionAssetCollectionMetadataMsg>,
) -> ContractResult<Response> {
    let contract = AssetContract::default();
    Ok(contract.instantiate_with_version(deps, &env, &info, msg, CONTRACT_NAME, CONTRACT_VERSION)?)
}

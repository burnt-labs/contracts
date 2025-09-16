use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use crate::{error::ContractResult, msg::{InstantiateMsg, XionAssetCollectionMetadataMsg}, CONTRACT_NAME, CONTRACT_VERSION};

pub type AssetContract<'a> = cw721::extension::Cw721Extensions<'a>;

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg<XionAssetCollectionMetadataMsg>,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
}

// Default implementation of the xion asset standard showing how to set up a contract
// to use the default trait XionAssetExecuteExtension
#[cfg(feature = "asset_base")]
use crate::msg::AssetExtensionQueryMsg;
use crate::traits::PluggableAsset;
use crate::{
    CONTRACT_NAME, CONTRACT_VERSION,
    error::ContractResult,
    msg::{AssetExtensionExecuteMsg, ExecuteMsg, InstantiateMsg},
    traits::DefaultAssetContract,
};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw721::{
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg, traits::Cw721Execute,
};
type AssetBaseContract<'a> = DefaultAssetContract<
    'a,
    DefaultOptionalNftExtension,
    DefaultOptionalNftExtensionMsg,
    DefaultOptionalCollectionExtension,
    DefaultOptionalCollectionExtensionMsg,
>;
#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
#[cfg(feature = "asset_base")]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg<DefaultOptionalCollectionExtensionMsg>,
) -> ContractResult<Response> {
    use crate::error::ContractError;

    let contract: AssetBaseContract<'static> = DefaultAssetContract::default();

    let response = contract
        .instantiate_with_version(deps, &env, &info, msg, CONTRACT_NAME, CONTRACT_VERSION)
        .map_err(Into::<ContractError>::into)?;

    Ok(response)
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
#[cfg(feature = "asset_base")]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<
        DefaultOptionalNftExtensionMsg,
        DefaultOptionalCollectionExtensionMsg,
        AssetExtensionExecuteMsg,
    >,
) -> ContractResult<Response> {
    let contract: AssetBaseContract<'static> = DefaultAssetContract::default();

    contract
        .execute_pluggable(deps, &env, &info, msg)
        .map_err(Into::into)
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
#[cfg(feature = "asset_base")]
pub fn query(
    deps: Deps,
    env: Env,
    msg: cw721::msg::Cw721QueryMsg<
        DefaultOptionalNftExtension,
        DefaultOptionalCollectionExtension,
        AssetExtensionQueryMsg,
    >,
) -> StdResult<Binary> {
    use cw721::traits::Cw721Query;

    use crate::error::ContractError;

    let contract: AssetBaseContract<'static> = DefaultAssetContract::default();

    contract
        .query(deps, &env, msg)
        .map_err(|err| ContractError::from(err).into())
}

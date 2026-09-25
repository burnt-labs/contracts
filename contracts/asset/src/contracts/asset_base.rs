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
/// `(cw2 name, cw2 version)` pairs this code accepts as a migration source.
///
/// The current version is listed deliberately, so migrating onto the code a contract
/// already runs is accepted rather than rejected. That is a no-op in practice: the
/// delegated cw721 migration rewrites the cw2 metadata to the same values and its legacy
/// steps do nothing once minter and creator are set. Allowing it keeps a redeploy of the
/// same code id, or a re-run of a migration that was interrupted, from needing a contract
/// change, and it costs nothing because a migration is already gated on the x/wasm admin.
/// Entries are added, never removed, so an older contract can always reach the newest code.
///
/// `asset-proxyable` is accepted so a collection can roll back to the base line; the base
/// never reads the variant's `trusted_proxies` storage, which stays dormant and is cleared
/// by the variant's own migrate if the collection ever moves back.
pub const MIGRATABLE_FROM: &[(&str, &str)] = &[
    (CONTRACT_NAME, "0.1.0"),
    (CONTRACT_NAME, "0.2.0"),
    ("asset-proxyable", "0.1.0"),
];

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

/// Migrate an existing `asset` contract to this code. cw721's migrate rewrites the cw2
/// metadata unconditionally, so the stored name and version are checked first.
#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
#[cfg(feature = "asset_base")]
pub fn migrate(
    deps: DepsMut,
    env: Env,
    msg: cw721::msg::Cw721MigrateMsg,
) -> ContractResult<Response> {
    use crate::error::ContractError;

    let stored = cw2::get_contract_version(deps.storage)?;
    if !MIGRATABLE_FROM.contains(&(stored.contract.as_str(), stored.version.as_str())) {
        return Err(ContractError::InvalidMigration {
            contract: stored.contract,
            version: stored.version,
        });
    }

    let contract: AssetBaseContract<'static> = DefaultAssetContract::default();
    let response = contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(response.add_attribute("from_contract", stored.contract))
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

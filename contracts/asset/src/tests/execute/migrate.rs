use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cw721::msg::Cw721MigrateMsg;

use crate::{
    CONTRACT_NAME, CONTRACT_VERSION, contracts::asset_base::migrate, error::ContractError,
};

use super::helpers::{expect_err, expect_ok};

fn no_update() -> Cw721MigrateMsg {
    Cw721MigrateMsg::WithUpdate {
        minter: None,
        creator: None,
    }
}

#[test]
fn migrate_from_previous_version_updates_metadata() {
    let mut deps = mock_dependencies();
    expect_ok(cw2::set_contract_version(
        deps.as_mut().storage,
        CONTRACT_NAME,
        "0.1.0",
    ));
    // a live contract always has minter and creator initialised
    let creator = deps.api.addr_make("creator");
    // a live contract always has collection info stored
    expect_ok(
        cw721::state::Cw721Config::<cosmwasm_std::Empty>::default()
            .collection_info
            .save(
                deps.as_mut().storage,
                &cw721::state::CollectionInfo {
                    name: "test".to_string(),
                    symbol: "TEST".to_string(),
                    updated_at: mock_env().block.time,
                },
            ),
    );
    {
        let api = deps.api;
        let storage = deps.as_mut().storage;
        expect_ok(cw721::state::CREATOR.initialize_owner(storage, &api, Some(creator.as_str())));
        expect_ok(cw721::state::MINTER.initialize_owner(storage, &api, Some(creator.as_str())));
    }

    expect_ok(migrate(deps.as_mut(), mock_env(), no_update()));

    let version = expect_ok(cw2::get_contract_version(deps.as_ref().storage));
    assert_eq!(version.contract, CONTRACT_NAME);
    assert_eq!(version.version, CONTRACT_VERSION);
    // roles untouched
    let stored = expect_ok(cw721::state::CREATOR.item.load(deps.as_ref().storage));
    assert_eq!(stored.owner, Some(creator));
}

#[test]
fn migrate_rejects_foreign_contract_or_unknown_version() {
    let mut deps = mock_dependencies();
    expect_ok(cw2::set_contract_version(
        deps.as_mut().storage,
        "something-else",
        "0.1.0",
    ));
    let err = expect_err(migrate(deps.as_mut(), mock_env(), no_update()));
    assert_eq!(
        err,
        ContractError::InvalidMigration {
            contract: "something-else".to_string(),
            version: "0.1.0".to_string()
        }
    );

    expect_ok(cw2::set_contract_version(
        deps.as_mut().storage,
        CONTRACT_NAME,
        "9.9.9",
    ));
    let err = expect_err(migrate(deps.as_mut(), mock_env(), no_update()));
    assert_eq!(
        err,
        ContractError::InvalidMigration {
            contract: CONTRACT_NAME.to_string(),
            version: "9.9.9".to_string()
        }
    );
}

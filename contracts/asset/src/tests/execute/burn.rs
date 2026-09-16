use cosmwasm_std::{
    Coin, Empty,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::{msg::Cw721ExecuteMsg, state::NftInfo};

use crate::{
    state::ListingInfo,
    traits::{DefaultAssetContract, PluggableAsset},
};

use super::helpers::expect_ok;

type TestContract<'a> = DefaultAssetContract<'a, Empty, Empty, Empty, Empty>;

fn seed_token(deps: cosmwasm_std::DepsMut, owner: &cosmwasm_std::Addr, token_id: &str) {
    let contract = TestContract::default();
    expect_ok(contract.config.cw721_config.increment_tokens(deps.storage));
    expect_ok(contract.config.cw721_config.nft_info.save(
        deps.storage,
        token_id,
        &NftInfo {
            owner: owner.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        },
    ));
}

fn seed_listing(deps: cosmwasm_std::DepsMut, seller: &cosmwasm_std::Addr, token_id: &str) {
    let contract = TestContract::default();
    expect_ok(contract.config.listings.save(
        deps.storage,
        token_id,
        &ListingInfo {
            id: token_id.to_string(),
            seller: seller.clone(),
            price: Coin::new(100_u128, "uxion"),
            reserved: None,
        },
    ));
}

fn burn_msg(token_id: &str) -> Cw721ExecuteMsg<Empty, Empty, crate::msg::AssetExtensionExecuteMsg> {
    Cw721ExecuteMsg::Burn {
        token_id: token_id.to_string(),
    }
}

#[test]
fn burn_is_rejected_while_listed() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = deps.api.addr_make("owner");
    seed_token(deps.as_mut(), &owner, "token-1");
    seed_listing(deps.as_mut(), &owner, "token-1");

    let contract = TestContract::default();
    let err = contract
        .execute_pluggable(
            deps.as_mut(),
            &env,
            &message_info(&owner, &[]),
            burn_msg("token-1"),
        )
        .expect_err("burn of a listed token must fail");
    assert_eq!(
        err.to_string(),
        cosmwasm_std::StdError::generic_err("cannot burn a token while it is listed").to_string()
    );

    // token and listing untouched
    assert!(
        contract
            .config
            .cw721_config
            .nft_info
            .may_load(deps.as_ref().storage, "token-1")
            .unwrap()
            .is_some()
    );
    assert!(
        contract
            .config
            .listings
            .may_load(deps.as_ref().storage, "token-1")
            .unwrap()
            .is_some()
    );
}

#[test]
fn burn_is_rejected_for_any_listing_even_a_stale_one() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let previous_owner = deps.api.addr_make("previous");
    let owner = deps.api.addr_make("owner");
    seed_token(deps.as_mut(), &owner, "token-1");
    // listing created by a previous owner: same rule as transfers, still blocks
    seed_listing(deps.as_mut(), &previous_owner, "token-1");

    let contract = TestContract::default();
    let err = contract
        .execute_pluggable(
            deps.as_mut(),
            &env,
            &message_info(&owner, &[]),
            burn_msg("token-1"),
        )
        .expect_err("burn of a listed token must fail");
    assert_eq!(
        err.to_string(),
        cosmwasm_std::StdError::generic_err("cannot burn a token while it is listed").to_string()
    );
    assert!(
        contract
            .config
            .cw721_config
            .nft_info
            .may_load(deps.as_ref().storage, "token-1")
            .unwrap()
            .is_some()
    );
}

#[test]
fn burn_succeeds_after_delist() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = deps.api.addr_make("owner");
    seed_token(deps.as_mut(), &owner, "token-1");
    seed_listing(deps.as_mut(), &owner, "token-1");

    let contract = TestContract::default();
    expect_ok(crate::execute::delist::<Empty, Empty>(
        deps.as_mut(),
        &env,
        &message_info(&owner, &[]),
        "token-1".to_string(),
    ));
    expect_ok(contract.execute_pluggable(
        deps.as_mut(),
        &env,
        &message_info(&owner, &[]),
        burn_msg("token-1"),
    ));
    assert!(
        contract
            .config
            .cw721_config
            .nft_info
            .may_load(deps.as_ref().storage, "token-1")
            .unwrap()
            .is_none()
    );
}

#[test]
fn burn_without_listing_is_unchanged() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let owner = deps.api.addr_make("owner");
    seed_token(deps.as_mut(), &owner, "token-1");

    let contract = TestContract::default();
    let response = expect_ok(contract.execute_pluggable(
        deps.as_mut(),
        &env,
        &message_info(&owner, &[]),
        burn_msg("token-1"),
    ));
    assert!(
        response
            .attributes
            .iter()
            .any(|a| a.key == "action" && a.value == "burn")
    );

    // a non-owner still cannot burn
    seed_token(deps.as_mut(), &owner, "token-2");
    let stranger = deps.api.addr_make("stranger");
    contract
        .execute_pluggable(
            deps.as_mut(),
            &env,
            &message_info(&stranger, &[]),
            burn_msg("token-2"),
        )
        .expect_err("stranger cannot burn");
}

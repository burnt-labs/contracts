use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw_multi_test::Executor;
use xion_nft_marketplace::helpers::query_listing;
use xion_nft_marketplace::msg::{ExecuteMsg, QueryMsg};
use xion_nft_marketplace::state::Listing;

#[test]
fn test_cancel_listing_success() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    // Setup contracts
    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Mint NFT to seller
    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Create listing
    let price = coin(100, "uxion");
    let listing_id = create_listing(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price,
    );

    // Cancel listing
    let cancel_msg = ExecuteMsg::CancelListing {
        listing_id: listing_id.clone(),
    };

    let result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &cancel_msg,
        &[],
    );

    assert!(result.is_ok());

    // Verify the listing was cancelled by checking events
    let events = result.unwrap().events;
    let cancel_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/cancel-listing");
    assert!(cancel_event.is_some());

    // listing should not be found on the asset contract
    let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
    assert!(listing_resp.is_err());

    // listing should not be found on the marketplace contract
    let listing_resp = app.wrap().query_wasm_smart::<Listing>(
        marketplace_contract.clone(),
        &QueryMsg::Listing {
            listing_id: listing_id.clone(),
        },
    );
    assert!(listing_resp.is_err());
}

#[test]
fn test_cancel_listing_unauthorized() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let unauthorized_user = app.api().addr_make("unauthorized");
    let manager = app.api().addr_make("manager");

    // Setup contracts
    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Mint NFT to seller
    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Create listing
    let price = coin(100, "uxion");
    let listing_id = create_listing(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price,
    );

    // Try to cancel listing with unauthorized user
    let cancel_msg = ExecuteMsg::CancelListing { listing_id };

    let result = app.execute_contract(
        unauthorized_user.clone(),
        marketplace_contract.clone(),
        &cancel_msg,
        &[],
    );

    assert!(result.is_err());

    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "sender is not the seller".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_cancel_listing_nonexistent() {
    let mut app = setup_app();
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    // Setup contracts
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Try to cancel non-existent listing
    let cancel_msg = ExecuteMsg::CancelListing {
        listing_id: "nonexistent".to_string(),
    };

    let result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &cancel_msg,
        &[],
    );

    assert!(result.is_err());
    result.unwrap_err().to_string().contains("not found");
}

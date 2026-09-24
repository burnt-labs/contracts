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

#[test]
fn test_cancel_reserved_listing_after_approval_revoked() {
    // A reserved_for listing holds the marketplace's own reservation on the
    // asset side, which only the reserver may clear while it is live. The
    // seller must still be able to cancel after revoking the marketplace's
    // CW721 approval, so cancellation acts as the reserver instead of
    // relying on that approval.
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price: coin(100, "uxion"),
        token_id: "token1".to_string(),
        reserved_for: Some(buyer.to_string()),
    };
    let list_result =
        app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(list_result.is_ok(), "{:?}", list_result.err());
    let listing_id = xion_nft_marketplace::helpers::generate_id(vec![
        asset_contract.as_bytes(),
        "token1".as_bytes(),
    ]);

    let revoke_msg = cw721_base::msg::ExecuteMsg::Revoke {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &revoke_msg, &[])
        .unwrap();

    let cancel_msg = ExecuteMsg::CancelListing {
        listing_id: listing_id.clone(),
    };
    let result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &cancel_msg,
        &[],
    );
    assert!(
        result.is_ok(),
        "seller must be able to cancel a reserved listing: {:?}",
        result.err()
    );

    let asset_listing = query_listing(&app.wrap(), &asset_contract, "token1");
    assert!(asset_listing.is_err(), "asset listing must be removed");

    let listing_resp = app
        .wrap()
        .query_wasm_smart::<Listing>(marketplace_contract, &QueryMsg::Listing { listing_id });
    assert!(listing_resp.is_err(), "marketplace listing must be removed");
}

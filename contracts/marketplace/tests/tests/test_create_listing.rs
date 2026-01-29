use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
use cw_multi_test::Executor;
use xion_nft_marketplace::helpers::generate_id;
use xion_nft_marketplace::helpers::query_listing;
use xion_nft_marketplace::msg::{ExecuteMsg, QueryMsg};
use xion_nft_marketplace::state::{Listing, ListingStatus};

#[test]
fn test_mint_nft() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let asset_contract = setup_asset_contract(&mut app, &minter);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");
}

#[test]
fn test_create_listing_success() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    // Setup contracts
    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Mint NFT to seller
    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Approve marketplace contract to manage the NFT
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Create listing
    let price = coin(100, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price: price.clone(),
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());
    let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
    assert!(listing_resp.is_ok());
    let listing = listing_resp.unwrap();

    // 100 - 2.5% fee = 97.5 floor
    assert_eq!(listing.price.amount.u128(), 97);
    assert_eq!(listing.seller, seller);

    // verify event is emitted
    let events = result.unwrap().events;
    let list_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item");
    assert!(list_event.is_some());
    let listing_id = generate_id(vec![&asset_contract.as_bytes(), &"token1".as_bytes()]);
    assert_eq!(
        "54f6c0b51fa2fae79401bc3d0f0e5d98f8be4588d312643f7b7dd631e88173cc",
        listing_id.clone()
    );
    let listing = app
        .wrap()
        .query_wasm_smart::<Listing>(
            marketplace_contract.clone(),
            &QueryMsg::Listing { listing_id },
        )
        .unwrap();

    assert_eq!(listing.price, price);
    assert_eq!(listing.seller, seller);
    assert_eq!(listing.status, ListingStatus::Active);
}

#[test]
fn test_create_listing_unauthorized() {
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

    // Try to create listing with unauthorized user
    let price = coin(100, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price,
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(
        unauthorized_user.clone(),
        marketplace_contract.clone(),
        &list_msg,
        &[],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "sender is not owner".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_create_listing_invalid_denom() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    // Setup contracts
    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Mint NFT to seller
    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Try to create listing with invalid denom
    let price = coin(100, "fakexion"); // Wrong denom
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price,
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidListingDenom {
            expected: "uxion".to_string(),
            actual: "fakexion".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_create_listing_already_listed() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    // Setup contracts
    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Mint NFT to seller
    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Approve marketplace contract to manage the NFT
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Create listing
    let price = coin(100, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price: price.clone(),
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());
    // Try to create second listing for same token
    let list_msg2 = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price,
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result2 = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &list_msg2,
        &[],
    );

    assert!(result2.is_err());
    assert_error(
        result2,
        xion_nft_marketplace::error::ContractError::AlreadyListed {}.to_string(),
    );
}

#[test]
fn test_create_listing_nonexistent_token() {
    let mut app = setup_app();
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");
    let minter = app.api().addr_make("minter");

    // Setup contracts
    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Try to create listing for non-existent token
    let price = coin(100, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price,
        token_id: "nonexistent".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "sender is not owner".to_string(),
        }
        .to_string(),
    );
}

use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw_multi_test::{BankSudo, Executor, SudoMsg};
use xion_nft_marketplace::msg::{ExecuteMsg, QueryMsg};
use xion_nft_marketplace::state::{Listing, ListingStatus};

#[test]
fn test_double_buy_blocked_when_approvals_stay_enabled() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer_a = app.api().addr_make("buyer_a");
    let buyer_b = app.api().addr_make("buyer_b");
    let manager = app.api().addr_make("manager");

    // Fund accounts
    for addr in [&buyer_a, &buyer_b, &seller, &minter, &manager] {
        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: addr.to_string(),
            amount: vec![coin(100_000, "uxion")],
        }))
        .unwrap();
    }

    let asset_contract = setup_asset_contract(&mut app, &minter);

    // Marketplace with sale_approvals ENABLED — stays enabled throughout
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(1000, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    // Buyer A buys -> pending sale created, listing status becomes Reserved
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };
    let result = app.execute_contract(
        buyer_a.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price.clone()],
    );
    assert!(
        result.is_ok(),
        "Buyer A's purchase should create a pending sale: {:?}",
        result.err()
    );

    // Verify listing is Reserved
    let listing: Listing = app
        .wrap()
        .query_wasm_smart(
            marketplace_contract.clone(),
            &QueryMsg::Listing {
                listing_id: listing_id.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        listing.status,
        ListingStatus::Reserved,
        "Listing should be Reserved after Buyer A's pending sale"
    );

    // Verify Buyer A's funds are escrowed in the marketplace
    let buyer_a_balance = app.wrap().query_balance(&buyer_a, "uxion").unwrap().amount;
    assert_eq!(buyer_a_balance.u128(), 100_000 - 1000);

    let marketplace_balance = app
        .wrap()
        .query_balance(&marketplace_contract, "uxion")
        .unwrap()
        .amount;
    assert_eq!(
        marketplace_balance.u128(),
        1000,
        "Marketplace should hold Buyer A's escrowed 1000 uxion"
    );

    // Buyer B attempts the same purchase — should be REJECTED
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };
    let result = app.execute_contract(
        buyer_b.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price.clone()],
    );

    assert!(
        result.is_err(),
        "Double-buy should be blocked when sale_approvals is enabled"
    );
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidListingStatus {
            expected: ListingStatus::Active.to_string(),
            actual: ListingStatus::Reserved.to_string(),
        }
        .to_string(),
    );

    // Verify state is unchanged after Buyer B's failed attempt:

    // 1. Listing still Reserved
    let listing: Listing = app
        .wrap()
        .query_wasm_smart(
            marketplace_contract.clone(),
            &QueryMsg::Listing {
                listing_id: listing_id.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        listing.status,
        ListingStatus::Reserved,
        "Listing should still be Reserved"
    );

    // 2. NFT still owned by seller (not transferred to anyone)
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(
            asset_contract.clone(),
            &cw721_base::msg::QueryMsg::OwnerOf {
                token_id: "token1".to_string(),
                include_expired: Some(false),
            },
        )
        .unwrap();
    assert_eq!(
        owner_resp.owner,
        seller.to_string(),
        "NFT should still be owned by the seller"
    );

    // 3. Buyer A's escrow untouched in marketplace
    let marketplace_balance = app
        .wrap()
        .query_balance(&marketplace_contract, "uxion")
        .unwrap()
        .amount;
    assert_eq!(
        marketplace_balance.u128(),
        1000,
        "Buyer A's escrowed funds should still be in the marketplace"
    );

    // 4. Buyer B's funds returned (tx reverted, they still have 100k)
    let buyer_b_balance = app.wrap().query_balance(&buyer_b, "uxion").unwrap().amount;
    assert_eq!(
        buyer_b_balance.u128(),
        100_000,
        "Buyer B should have all funds back after rejected purchase"
    );
}

#[test]
fn test_double_buy_blocked_when_approvals_toggled_off() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer_a = app.api().addr_make("buyer_a");
    let buyer_b = app.api().addr_make("buyer_b");
    let manager = app.api().addr_make("manager");

    // Fund accounts
    for addr in [&buyer_a, &buyer_b, &seller, &minter, &manager] {
        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: addr.to_string(),
            amount: vec![coin(100_000, "uxion")],
        }))
        .unwrap();
    }

    let asset_contract = setup_asset_contract(&mut app, &minter);

    // Step 1: Instantiate marketplace with sale_approvals ENABLED
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    // Step 2: Mint and list an NFT
    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(1000, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    // Step 3: Buyer A buys -> creates a pending sale, listing becomes Reserved
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };
    let result = app.execute_contract(
        buyer_a.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price.clone()],
    );
    assert!(
        result.is_ok(),
        "Buyer A's purchase should create a pending sale: {:?}",
        result.err()
    );

    // Verify listing is Reserved and Buyer A's funds are escrowed
    let listing: Listing = app
        .wrap()
        .query_wasm_smart(
            marketplace_contract.clone(),
            &QueryMsg::Listing {
                listing_id: listing_id.clone(),
            },
        )
        .unwrap();
    assert_eq!(listing.status, ListingStatus::Reserved);

    let marketplace_balance = app
        .wrap()
        .query_balance(&marketplace_contract, "uxion")
        .unwrap()
        .amount;
    assert_eq!(marketplace_balance.u128(), 1000);

    // Step 4: Manager flips sale_approvals to false
    let update_config_msg = ExecuteMsg::UpdateConfig {
        config: serde_json::from_value(serde_json::json!({
            "manager": manager.to_string(),
            "fee_recipient": manager.to_string(),
            "sale_approvals": false,
            "fee_bps": 250,
            "listing_denom": "uxion"
        }))
        .unwrap(),
    };
    app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &update_config_msg,
        &[],
    )
    .unwrap();

    // Step 5: Buyer B buys the same Reserved listing
    // This SHOULD fail because the listing is Reserved (Buyer A's pending sale exists)
    // but execute_buy_item never checks listing.status
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };
    let result = app.execute_contract(
        buyer_b.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price.clone()],
    );

    assert!(
        result.is_err(),
        "BUG: Buyer B was able to buy a Reserved listing after manager toggled \
         sale_approvals off. This is a double-spend — Buyer A's escrowed 1000 uxion \
         is locked in the contract while the NFT was sold to Buyer B. \
         Fix: add `listing.status != Active` check in execute_buy_item."
    );
}

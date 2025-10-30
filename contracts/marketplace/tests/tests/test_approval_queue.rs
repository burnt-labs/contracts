use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw721_base::msg::QueryMsg as OwnerQueryMsg;
use cw_multi_test::Executor;
use xion_nft_marketplace::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use xion_nft_marketplace::state::{Listing, ListingStatus, PendingSale};

#[test]
fn test_buy_with_approvals_disabled() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price],
    );

    assert!(result.is_ok());

    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());

    let listing_query = app.wrap().query_wasm_smart::<Listing>(
        marketplace_contract.clone(),
        &QueryMsg::Listing { listing_id },
    );
    assert!(listing_query.is_err());
}

#[test]
fn test_buy_with_approvals_enabled_creates_pending_sale() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );

    assert!(result.is_ok());

    let events = result.unwrap().events;
    let pending_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/pending-sale-created")
        .expect("pending-sale-created event should be emitted");

    let pending_sale_id = pending_event
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    let pending_sale: PendingSale = app
        .wrap()
        .query_wasm_smart(
            marketplace_contract.clone(),
            &QueryMsg::PendingSale {
                id: pending_sale_id.clone(),
            },
        )
        .unwrap();

    assert_eq!(pending_sale.buyer, buyer);
    assert_eq!(pending_sale.seller, seller);
    assert_eq!(pending_sale.price, price);

    let listing: Listing = app
        .wrap()
        .query_wasm_smart(
            marketplace_contract.clone(),
            &QueryMsg::Listing { listing_id },
        )
        .unwrap();

    assert_eq!(listing.status, ListingStatus::Reserved);

    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, seller.to_string());
}

#[test]
fn test_approve_sale_success() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let buy_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );

    let pending_sale_id = buy_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/pending-sale-created")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    let approve_msg = ExecuteMsg::ApproveSale {
        id: pending_sale_id.clone(),
    };

    let result = app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &approve_msg,
        &[],
    );

    assert!(result.is_ok());

    let events = result.unwrap().events;
    let approved_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/sale-approved");
    assert!(approved_event.is_some());

    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());

    let listing_query = app.wrap().query_wasm_smart::<Listing>(
        marketplace_contract.clone(),
        &QueryMsg::Listing { listing_id },
    );
    assert!(listing_query.is_err());

    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(pending_sale_query.is_err());
}

#[test]
fn test_approve_sale_unauthorized() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let unauthorized = app.api().addr_make("unauthorized");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id,
        price: price.clone(),
    };

    let buy_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price],
    );

    let pending_sale_id = buy_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/pending-sale-created")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    let approve_msg = ExecuteMsg::ApproveSale {
        id: pending_sale_id,
    };

    let result = app.execute_contract(
        unauthorized.clone(),
        marketplace_contract.clone(),
        &approve_msg,
        &[],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "sender is not manager".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_reject_sale_success() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buyer_balance_before = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let buy_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );

    let pending_sale_id = buy_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/pending-sale-created")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    let reject_msg = ExecuteMsg::RejectSale {
        id: pending_sale_id.clone(),
    };

    let result = app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &reject_msg,
        &[],
    );

    assert!(result.is_ok());

    let events = result.unwrap().events;
    let rejected_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/sale-rejected");
    assert!(rejected_event.is_some());

    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    assert_eq!(buyer_balance_before, buyer_balance_after);

    let listing_query = app.wrap().query_wasm_smart::<Listing>(
        marketplace_contract.clone(),
        &QueryMsg::Listing { listing_id },
    );
    assert!(
        listing_query.is_err(),
        "Listing should be deleted after rejection"
    );

    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, seller.to_string());

    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(pending_sale_query.is_err());
}

#[test]
fn test_reject_sale_unauthorized() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let unauthorized = app.api().addr_make("unauthorized");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id,
        price: price.clone(),
    };

    let buy_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price],
    );

    let pending_sale_id = buy_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/pending-sale-created")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    let reject_msg = ExecuteMsg::RejectSale {
        id: pending_sale_id,
    };

    let result = app.execute_contract(
        unauthorized.clone(),
        marketplace_contract.clone(),
        &reject_msg,
        &[],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "sender is not manager".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_cannot_cancel_reserved_listing() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price],
    )
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

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidListingStatus {
            expected: "Active".to_string(),
            actual: "Reserved".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_approve_sale_fee_routing() {
    // This test verifies that the marketplace fee routing bug is fixed
    // Expected behavior:
    // - Asset contract receives asset_price (price - marketplace_fee)
    // - Manager receives marketplace_fee
    // - Seller receives asset_price from asset contract
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Price: 1000 uxion
    // Fee BPS: 250 (2.5%)
    // Expected marketplace fee: 25 uxion
    // Expected asset price: 975 uxion
    let price = coin(1000, "uxion");
    let expected_marketplace_fee = cosmwasm_std::Uint128::from(25u128);
    let expected_asset_price = cosmwasm_std::Uint128::from(975u128);

    // Capture initial balances
    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let buyer_balance_before = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;

    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    // Buyer creates pending sale (sends funds to marketplace)
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let buy_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );

    assert!(buy_result.is_ok());

    // Extract pending sale ID
    let pending_sale_id = buy_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/pending-sale-created")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Verify buyer's funds are escrowed in marketplace
    let buyer_balance_after_buy = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    assert_eq!(
        buyer_balance_after_buy,
        buyer_balance_before - price.amount,
        "Buyer funds should be escrowed in marketplace"
    );

    // Manager approves the sale
    let approve_msg = ExecuteMsg::ApproveSale {
        id: pending_sale_id.clone(),
    };

    let approve_result = app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &approve_msg,
        &[],
    );

    assert!(approve_result.is_ok());

    // Check balances after approval
    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;

    // Verify seller received asset_price (price - marketplace_fee)
    assert_eq!(
        seller_balance_after,
        seller_balance_before + expected_asset_price,
        "Seller should receive asset_price (price - marketplace_fee)"
    );

    // Verify manager received marketplace fee
    assert_eq!(
        manager_balance_after,
        manager_balance_before + expected_marketplace_fee,
        "Manager should receive marketplace fee"
    );

    // Verify buyer balance hasn't changed since they already paid
    assert_eq!(
        buyer_balance_after,
        buyer_balance_after_buy,
        "Buyer balance should not change during approval"
    );

    // Verify NFT ownership transferred to buyer
    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());

    // Verify listing and pending sale are cleaned up
    let listing_query = app.wrap().query_wasm_smart::<Listing>(
        marketplace_contract.clone(),
        &QueryMsg::Listing { listing_id },
    );
    assert!(listing_query.is_err(), "Listing should be deleted after approval");

    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(pending_sale_query.is_err(), "Pending sale should be deleted after approval");
}

#[test]
fn test_approve_sale_fee_routing_with_zero_fee() {
    // Edge case: Test with zero marketplace fee
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);

    // Create marketplace with zero fee
    let marketplace_code_id = app.store_code(marketplace_contract());
    let config_json = serde_json::json!({
        "manager": manager.to_string(),
        "fee_recipient": manager.to_string(),
        "sale_approvals": true,
        "fee_bps": 0,  // Zero fee
        "listing_denom": "uxion"
    });
    let instantiate_msg = InstantiateMsg {
        config: serde_json::from_value(config_json).unwrap(),
    };
    let marketplace_contract = app.instantiate_contract(
        marketplace_code_id,
        manager.clone(),
        &instantiate_msg,
        &[],
        "test-marketplace-zero-fee",
        None,
    ).unwrap();

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(1000, "uxion");

    // Capture initial balances
    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;

    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let buy_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );

    let pending_sale_id = buy_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/pending-sale-created")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    let approve_msg = ExecuteMsg::ApproveSale {
        id: pending_sale_id,
    };

    let approve_result = app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &approve_msg,
        &[],
    );

    if let Err(ref e) = approve_result {
        println!("Approve sale error with zero fee: {:?}", e);
    }
    assert!(approve_result.is_ok());

    // Check balances
    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;

    // With zero fee, seller should receive full price
    assert_eq!(
        seller_balance_after,
        seller_balance_before + price.amount,
        "Seller should receive full price with zero fee"
    );

    // Manager should receive zero fee (balance unchanged except for gas)
    assert_eq!(
        manager_balance_after,
        manager_balance_before,
        "Manager should not receive any fee with zero fee_bps"
    );

    // Verify NFT ownership transferred
    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract, &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());
}

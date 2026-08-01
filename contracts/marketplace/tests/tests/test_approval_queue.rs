use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw721_base::msg::QueryMsg as OwnerQueryMsg;
use cw_multi_test::Executor;
use xion_nft_marketplace::helpers::query_listing;
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
fn test_pending_sale_reservation_blocks_direct_asset_buy() {
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
        listing_id,
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );
    assert!(result.is_ok());

    let asset_listing = query_listing(&app.wrap(), &asset_contract, "token1").unwrap();
    let reserved = asset_listing
        .reserved
        .expect("asset listing should be reserved");
    assert_eq!(reserved.reserver, marketplace_contract);

    let direct_buy_msg = asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        asset::msg::AssetExtensionExecuteMsg,
    >::UpdateExtension {
        msg: asset::msg::AssetExtensionExecuteMsg::Buy {
            token_id: "token1".to_string(),
            recipient: None,
        },
    };

    let direct_buy_result = app.execute_contract(
        buyer.clone(),
        asset_contract.clone(),
        &direct_buy_msg,
        std::slice::from_ref(&price),
    );
    assert!(direct_buy_result.is_err());
    assert_error(
        direct_buy_result,
        "Generic error: Generic error: Unauthorized".to_string(),
    );
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
fn test_approve_sale_with_recipient() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let recipient = app.api().addr_make("recipient");

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

    let finalize_for = ExecuteMsg::FinalizeFor {
        listing_id: listing_id.clone(),
        price: price.clone(),
        recipient: recipient.to_string(),
    };

    let buy_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &finalize_for,
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
    assert_eq!(owner_resp.owner, recipient.to_string());

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
fn test_reject_sale_after_manual_unreserve() {
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

    // Buyer purchases, creating a pending sale
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

    // Manually unreserve and delist
    let unreserve_msg = asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        asset::msg::AssetExtensionExecuteMsg,
    >::UpdateExtension {
        msg: asset::msg::AssetExtensionExecuteMsg::UnReserve {
            token_id: "token1".to_string(),
            delist: Some(true),
        },
    };

    // Use seller (owner) to unreserve since they have list permissions
    let unreserve_result =
        app.execute_contract(seller.clone(), asset_contract.clone(), &unreserve_msg, &[]);

    assert!(unreserve_result.is_ok(), "Unreserve should succeed");

    // Now manager rejects the sale
    let reject_msg = ExecuteMsg::RejectSale {
        id: pending_sale_id.clone(),
    };

    let result = app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &reject_msg,
        &[],
    );

    assert!(
        result.is_ok(),
        "Rejection should succeed even after manual unreserve"
    );

    let events = result.unwrap().events;
    let rejected_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/sale-rejected");
    assert!(
        rejected_event.is_some(),
        "Sale rejected event should be emitted"
    );

    // Verify buyer gets refunded
    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    assert_eq!(
        buyer_balance_before, buyer_balance_after,
        "Buyer should receive full refund"
    );

    // Verify listing is deleted
    let listing_query = app.wrap().query_wasm_smart::<Listing>(
        marketplace_contract.clone(),
        &QueryMsg::Listing { listing_id },
    );
    assert!(
        listing_query.is_err(),
        "Listing should be deleted after rejection"
    );

    // Verify NFT ownership is still with seller
    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(
        owner_resp.owner,
        seller.to_string(),
        "NFT should still be owned by seller"
    );

    // Verify pending sale is removed
    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(
        pending_sale_query.is_err(),
        "Pending sale should be removed after rejection"
    );
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
        buyer_balance_after, buyer_balance_after_buy,
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
    assert!(
        listing_query.is_err(),
        "Listing should be deleted after approval"
    );

    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(
        pending_sale_query.is_err(),
        "Pending sale should be deleted after approval"
    );
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
    let marketplace_contract = app
        .instantiate_contract(
            marketplace_code_id,
            manager.clone(),
            &instantiate_msg,
            &[],
            "test-marketplace-zero-fee",
            None,
        )
        .unwrap();

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
        manager_balance_after, manager_balance_before,
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

#[test]
fn test_approve_sale_fee_routing_with_royalties() {
    // This test verifies proper fee routing with BOTH marketplace fees AND royalties
    // in the sale approval flow
    // Expected flow:
    // 1. Buyer pays: 1000 uxion (escrowed in marketplace)
    // 2. Manager approves sale
    // 3. Marketplace fee (2.5% of 1000): 25 uxion → Manager
    // 4. Asset price sent to asset contract: 975 uxion
    // 5. Asset contract applies royalty (5% of 975): 48 uxion → Royalty recipient
    // 6. Seller receives: 927 uxion (975 - 48)
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let royalty_recipient = app.api().addr_make("royalty_recipient");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    // Set up 5% royalty on the asset contract
    let royalty_plugin = asset::plugin::Plugin::Royalty {
        bps: 500, // 5%
        recipient: royalty_recipient.clone(),
    };

    let set_plugin_msg = asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        asset::msg::AssetExtensionExecuteMsg,
    >::UpdateExtension {
        msg: asset::msg::AssetExtensionExecuteMsg::SetCollectionPlugin {
            plugins: vec![royalty_plugin],
        },
    };

    app.execute_contract(minter.clone(), asset_contract.clone(), &set_plugin_msg, &[])
        .unwrap();

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Price: 1000 uxion
    // Marketplace fee: 25 uxion (2.5%)
    // Asset price: 975 uxion
    // Royalty: 48 uxion (5% of 975)
    // Seller receives: 927 uxion
    let price = coin(1000, "uxion");
    let expected_marketplace_fee = cosmwasm_std::Uint128::from(25u128);
    let expected_royalty = cosmwasm_std::Uint128::from(48u128);
    let expected_seller_payment = cosmwasm_std::Uint128::from(927u128);

    // Capture initial balances
    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let buyer_balance_before = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    let royalty_balance_before = app
        .wrap()
        .query_balance(&royalty_recipient, "uxion")
        .unwrap()
        .amount;

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
    let royalty_balance_after = app
        .wrap()
        .query_balance(&royalty_recipient, "uxion")
        .unwrap()
        .amount;

    // Verify seller received (975 - 48) = 927
    assert_eq!(
        seller_balance_after,
        seller_balance_before + expected_seller_payment,
        "Seller should receive 927 uxion (975 asset_price - 48 royalty)"
    );

    // Verify manager received marketplace fee (25)
    assert_eq!(
        manager_balance_after,
        manager_balance_before + expected_marketplace_fee,
        "Manager should receive 25 uxion marketplace fee"
    );

    // Verify royalty recipient received 5% of asset price (48)
    assert_eq!(
        royalty_balance_after,
        royalty_balance_before + expected_royalty,
        "Royalty recipient should receive 48 uxion (5% of 975)"
    );

    // Verify buyer balance hasn't changed since they already paid
    assert_eq!(
        buyer_balance_after, buyer_balance_after_buy,
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
    assert!(
        listing_query.is_err(),
        "Listing should be deleted after approval"
    );

    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(
        pending_sale_query.is_err(),
        "Pending sale should be deleted after approval"
    );
}

#[test]
fn test_approve_sale_with_existing_pending_sale() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer1 = app.api().addr_make("buyer1");
    let buyer2 = app.api().addr_make("buyer2");
    let manager = app.api().addr_make("manager");

    // Give funds to buyer1 and buyer2
    use cw_multi_test::{BankSudo, SudoMsg};
    let funds = vec![coin(10000, "uxion")];
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: buyer1.to_string(),
        amount: funds.clone(),
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: buyer2.to_string(),
        amount: funds.clone(),
    }))
    .unwrap();

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    // Use a unique token_id with timestamp to avoid any potential conflicts
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let token_id = format!("test_approve_existing_pending_{}", timestamp);
    mint_nft(&mut app, &asset_contract, &minter, &seller, &token_id);

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        &token_id,
        price.clone(),
    );

    // First buyer creates a pending sale
    let buy_msg1 = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let buy_result1 = app.execute_contract(
        buyer1.clone(),
        marketplace_contract.clone(),
        &buy_msg1,
        std::slice::from_ref(&price),
    );

    assert!(
        buy_result1.is_ok(),
        "First buy should succeed: {:?}",
        buy_result1.as_ref().unwrap_err()
    );

    let pending_sale_id1 = buy_result1
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

    // Verify first pending sale exists
    let pending_sale1: PendingSale = app
        .wrap()
        .query_wasm_smart(
            marketplace_contract.clone(),
            &QueryMsg::PendingSale {
                id: pending_sale_id1.clone(),
            },
        )
        .unwrap();
    assert_eq!(pending_sale1.buyer, buyer1);
    assert_eq!(pending_sale1.token_id, token_id.clone());

    // Second buyer tries to create another pending sale for the same item - should fail
    let buy_msg2 = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let buy_result2 = app.execute_contract(
        buyer2.clone(),
        marketplace_contract.clone(),
        &buy_msg2,
        std::slice::from_ref(&price),
    );

    assert!(buy_result2.is_err());
    assert_error(
        buy_result2,
        xion_nft_marketplace::error::ContractError::InvalidListingStatus {
            expected: "Active".to_string(),
            actual: "Reserved".to_string(),
        }
        .to_string(),
    );

    // Now approve the first pending sale
    let approve_msg = ExecuteMsg::ApproveSale {
        id: pending_sale_id1.clone(),
    };

    let approve_result = app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &approve_msg,
        &[],
    );

    assert!(
        approve_result.is_ok(),
        "Approval should succeed even with existing pending sale"
    );

    let events = approve_result.unwrap().events;
    let approved_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/sale-approved");
    assert!(
        approved_event.is_some(),
        "Sale approved event should be emitted"
    );

    // Verify the pending sale is removed after approval
    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id1,
        },
    );
    assert!(
        pending_sale_query.is_err(),
        "Pending sale should be removed after approval"
    );

    // Verify the listing is removed
    let listing_query = app.wrap().query_wasm_smart::<Listing>(
        marketplace_contract.clone(),
        &QueryMsg::Listing { listing_id },
    );
    assert!(
        listing_query.is_err(),
        "Listing should be deleted after approval"
    );

    // Verify NFT ownership transferred to buyer1
    let owner_query = OwnerQueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer1.to_string());
}

#[test]
fn test_approve_expired_pending_sale_fails() {
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

    // Buyer creates pending sale
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

    // Advance block time past the 24-hour expiration
    app.update_block(|block| {
        block.time = block.time.plus_seconds(86401);
    });

    // Manager tries to approve the expired sale
    let approve_msg = ExecuteMsg::ApproveSale {
        id: pending_sale_id.clone(),
    };

    let result = app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &approve_msg,
        &[],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::PendingSaleExpired {
            id: pending_sale_id,
        }
        .to_string(),
    );

    // Verify NFT is still owned by seller (sale was not executed)
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
fn test_reclaim_expired_sale_success() {
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
    assert!(buy_result.is_ok());

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

    // Advance block time past the 24-hour expiration
    app.update_block(|block| {
        block.time = block.time.plus_seconds(86401);
    });

    // Buyer reclaims the expired sale
    let reclaim_msg = ExecuteMsg::ReclaimExpiredSale {
        id: pending_sale_id.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &reclaim_msg,
        &[],
    );
    assert!(result.is_ok());

    // Verify the event has reason "expired"
    let events = result.unwrap().events;
    let rejected_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/sale-rejected")
        .expect("sale-rejected event should be emitted");
    let reason = rejected_event
        .attributes
        .iter()
        .find(|a| a.key == "reason")
        .unwrap();
    assert_eq!(reason.value, "expired");

    // Verify buyer is refunded
    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    assert_eq!(buyer_balance_before, buyer_balance_after);

    // Verify pending sale is removed
    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(pending_sale_query.is_err());

    // Verify NFT still owned by seller
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
fn test_reclaim_expired_sale_not_yet_expired() {
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
    assert!(buy_result.is_ok());

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

    // Do NOT advance time — sale is still active

    let reclaim_msg = ExecuteMsg::ReclaimExpiredSale {
        id: pending_sale_id.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &reclaim_msg,
        &[],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::PendingSaleNotExpired {
            id: pending_sale_id,
        }
        .to_string(),
    );
}

#[test]
fn test_reclaim_expired_sale_unauthorized() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let random = app.api().addr_make("random");

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
    assert!(buy_result.is_ok());

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

    // Advance past expiry
    app.update_block(|block| {
        block.time = block.time.plus_seconds(86401);
    });

    // Random user tries to reclaim — should fail
    let reclaim_msg = ExecuteMsg::ReclaimExpiredSale {
        id: pending_sale_id,
    };

    let result = app.execute_contract(
        random.clone(),
        marketplace_contract.clone(),
        &reclaim_msg,
        &[],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "only the buyer can reclaim an expired sale".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_reject_sale_emits_reason() {
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
    assert!(buy_result.is_ok());

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
        manager.clone(),
        marketplace_contract.clone(),
        &reject_msg,
        &[],
    );
    assert!(result.is_ok());

    let events = result.unwrap().events;
    let rejected_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/sale-rejected")
        .expect("sale-rejected event should be emitted");
    let reason = rejected_event
        .attributes
        .iter()
        .find(|a| a.key == "reason")
        .unwrap();
    assert_eq!(reason.value, "rejected_by_manager");
}

#[test]
fn test_manager_can_reject_expired_sale() {
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
    assert!(buy_result.is_ok());

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

    // Advance past expiry
    app.update_block(|block| {
        block.time = block.time.plus_seconds(86401);
    });

    // Manager can still reject even after expiry
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

    // Verify buyer is refunded
    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    assert_eq!(buyer_balance_before, buyer_balance_after);

    // Verify pending sale is removed
    let pending_sale_query = app.wrap().query_wasm_smart::<PendingSale>(
        marketplace_contract.clone(),
        &QueryMsg::PendingSale {
            id: pending_sale_id,
        },
    );
    assert!(pending_sale_query.is_err());
}

use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw721_base::msg::QueryMsg as OwnerQueryMsg;
use cw_multi_test::Executor;
use xion_nft_marketplace::msg::ExecuteMsg;

#[test]
fn test_accept_offer_fee_routing() {
    // This test verifies that marketplace fees are correctly applied when accepting offers
    // Expected behavior:
    // - Asset contract receives asset_price (offer_price - marketplace_fee)
    // - Manager receives marketplace_fee
    // - Seller receives asset_price from asset contract
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Offer price: 1000 uxion
    // Fee BPS: 250 (2.5%)
    // Expected marketplace fee: 25 uxion
    // Expected asset price: 975 uxion
    let offer_price = coin(1000, "uxion");
    let expected_marketplace_fee = cosmwasm_std::Uint128::from(25u128);
    let expected_asset_price = cosmwasm_std::Uint128::from(975u128);

    // Capture initial balances
    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let buyer_balance_before = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;

    // Buyer creates offer (sends funds to marketplace)
    let create_offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let create_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    assert!(create_result.is_ok());

    // Extract offer ID from events
    let offer_id = create_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Verify buyer's funds are escrowed in marketplace
    let buyer_balance_after_offer = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    assert_eq!(
        buyer_balance_after_offer,
        buyer_balance_before - offer_price.amount,
        "Buyer funds should be escrowed in marketplace"
    );

    // Seller must approve marketplace to manage the NFT before accepting offer
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Seller accepts the offer
    let accept_msg = ExecuteMsg::AcceptOffer {
        id: offer_id.clone(),
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let accept_result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_msg,
        &[],
    );

    if let Err(ref e) = accept_result {
        println!("Accept offer error: {:?}", e);
    }
    assert!(accept_result.is_ok());

    // Check balances after acceptance
    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;

    // Verify seller received asset_price (offer_price - marketplace_fee)
    assert_eq!(
        seller_balance_after,
        seller_balance_before + expected_asset_price,
        "Seller should receive asset_price (offer_price - marketplace_fee)"
    );

    // Verify manager received marketplace fee
    assert_eq!(
        manager_balance_after,
        manager_balance_before + expected_marketplace_fee,
        "Manager should receive marketplace fee"
    );

    // Verify buyer balance hasn't changed since they already paid
    assert_eq!(
        buyer_balance_after, buyer_balance_after_offer,
        "Buyer balance should not change during offer acceptance"
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
}

#[test]
fn test_accept_offer_fee_routing_with_zero_fee() {
    // Edge case: Test offer acceptance with zero marketplace fee
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
        "sale_approvals": false,
        "fee_bps": 0,  // Zero fee
        "listing_denom": "uxion"
    });
    let instantiate_msg = xion_nft_marketplace::msg::InstantiateMsg {
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

    let offer_price = coin(1000, "uxion");

    // Capture initial balances
    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;

    // Buyer creates offer
    let create_offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let create_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    let offer_id = create_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Seller must approve marketplace to manage the NFT
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Seller accepts the offer
    let accept_msg = ExecuteMsg::AcceptOffer {
        id: offer_id,
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let accept_result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_msg,
        &[],
    );

    assert!(accept_result.is_ok());

    // Check balances
    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;

    // With zero fee, seller should receive full offer price
    assert_eq!(
        seller_balance_after,
        seller_balance_before + offer_price.amount,
        "Seller should receive full offer price with zero fee"
    );

    // Manager should receive zero fee (balance unchanged)
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
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());
}

#[test]
fn test_accept_collection_offer_fee_routing() {
    // This test verifies that marketplace fees are correctly applied when accepting collection offers
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Collection offer price: 1000 uxion
    // Fee BPS: 250 (2.5%)
    // Expected marketplace fee: 25 uxion
    // Expected asset price: 975 uxion
    let offer_price = coin(1000, "uxion");
    let expected_marketplace_fee = cosmwasm_std::Uint128::from(25u128);
    let expected_asset_price = cosmwasm_std::Uint128::from(975u128);

    // Capture initial balances
    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let buyer_balance_before = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;

    // Buyer creates collection offer
    let create_offer_msg = ExecuteMsg::CreateCollectionOffer {
        collection: asset_contract.to_string(),
        price: offer_price.clone(),
    };

    let create_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    assert!(create_result.is_ok());

    // Extract offer ID from events
    let offer_id = create_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-collection-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Verify buyer's funds are escrowed
    let buyer_balance_after_offer = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    assert_eq!(
        buyer_balance_after_offer,
        buyer_balance_before - offer_price.amount,
        "Buyer funds should be escrowed in marketplace"
    );

    // Seller must approve marketplace to manage the NFT
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Seller accepts the collection offer
    let accept_msg = ExecuteMsg::AcceptCollectionOffer {
        id: offer_id.clone(),
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let accept_result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_msg,
        &[],
    );

    assert!(accept_result.is_ok());

    // Check balances after acceptance
    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;

    // Verify seller received asset_price
    assert_eq!(
        seller_balance_after,
        seller_balance_before + expected_asset_price,
        "Seller should receive asset_price (offer_price - marketplace_fee)"
    );

    // Verify manager received marketplace fee
    assert_eq!(
        manager_balance_after,
        manager_balance_before + expected_marketplace_fee,
        "Manager should receive marketplace fee"
    );

    // Verify buyer balance hasn't changed
    assert_eq!(
        buyer_balance_after, buyer_balance_after_offer,
        "Buyer balance should not change during offer acceptance"
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
}

#[test]
fn test_accept_collection_offer_fee_routing_with_zero_fee() {
    // Edge case: Test collection offer acceptance with zero marketplace fee
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
        "sale_approvals": false,
        "fee_bps": 0,  // Zero fee
        "listing_denom": "uxion"
    });
    let instantiate_msg = xion_nft_marketplace::msg::InstantiateMsg {
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

    let offer_price = coin(1000, "uxion");

    // Capture initial balances
    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;

    // Buyer creates collection offer
    let create_offer_msg = ExecuteMsg::CreateCollectionOffer {
        collection: asset_contract.to_string(),
        price: offer_price.clone(),
    };

    let create_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    let offer_id = create_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-collection-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Seller must approve marketplace to manage the NFT
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Seller accepts the collection offer
    let accept_msg = ExecuteMsg::AcceptCollectionOffer {
        id: offer_id,
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let accept_result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_msg,
        &[],
    );

    assert!(accept_result.is_ok());

    // Check balances
    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;

    // With zero fee, seller should receive full offer price
    assert_eq!(
        seller_balance_after,
        seller_balance_before + offer_price.amount,
        "Seller should receive full offer price with zero fee"
    );

    // Manager should receive zero fee
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
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());
}

#[test]
fn test_accept_offer_fee_routing_with_royalties() {
    // This test verifies proper fee routing with BOTH marketplace fees AND royalties
    // Expected flow:
    // 1. Offer price: 1000 uxion
    // 2. Marketplace fee (2.5% of 1000): 25 uxion → Manager
    // 3. Asset price sent to asset contract: 975 uxion
    // 4. Asset contract applies royalty (5% of 975): 48 uxion → Royalty recipient
    // 5. Seller receives: 927 uxion (975 - 48)
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let royalty_recipient = app.api().addr_make("royalty_recipient");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

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

    // Offer price: 1000 uxion
    // Marketplace fee: 25 uxion (2.5% of 1000)
    // Asset price: 975 uxion
    // Royalty: 48 uxion (5% of 975, rounded down)
    // Seller receives: 927 uxion
    let offer_price = coin(1000, "uxion");
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

    // Buyer creates offer
    let create_offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let create_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    assert!(create_result.is_ok());

    let offer_id = create_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Seller must approve marketplace to manage the NFT
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Seller accepts the offer
    let accept_msg = ExecuteMsg::AcceptOffer {
        id: offer_id.clone(),
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let accept_result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_msg,
        &[],
    );

    assert!(accept_result.is_ok());

    // Check balances after acceptance
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

    // Verify buyer paid full offer price
    assert_eq!(
        buyer_balance_after,
        buyer_balance_before - offer_price.amount,
        "Buyer should have paid full offer price of 1000 uxion"
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
}

#[test]
fn test_accept_collection_offer_fee_routing_with_royalties() {
    // This test verifies proper fee routing with BOTH marketplace fees AND royalties
    // for collection offers
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let royalty_recipient = app.api().addr_make("royalty_recipient");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

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

    // Collection offer price: 1000 uxion
    // Marketplace fee: 25 uxion (2.5%)
    // Asset price: 975 uxion
    // Royalty: 48 uxion (5% of 975)
    // Seller receives: 927 uxion
    let offer_price = coin(1000, "uxion");
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

    // Buyer creates collection offer
    let create_offer_msg = ExecuteMsg::CreateCollectionOffer {
        collection: asset_contract.to_string(),
        price: offer_price.clone(),
    };

    let create_result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    assert!(create_result.is_ok());

    let offer_id = create_result
        .unwrap()
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-collection-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Seller must approve marketplace to manage the NFT
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Seller accepts the collection offer
    let accept_msg = ExecuteMsg::AcceptCollectionOffer {
        id: offer_id.clone(),
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let accept_result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_msg,
        &[],
    );

    assert!(accept_result.is_ok());

    // Check balances after acceptance
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

    // Verify buyer paid full offer price
    assert_eq!(
        buyer_balance_after,
        buyer_balance_before - offer_price.amount,
        "Buyer should have paid full offer price of 1000 uxion"
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
}

#[test]
fn test_create_offer_disabled_with_approvals() {
    // This test verifies that creating offers is disabled when sale approvals are enabled
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let offer_price = coin(1000, "uxion");

    // Try to create offer - should fail
    let create_offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    // Should fail with OfferesDisabled error
    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::OfferesDisabled {}.to_string(),
    );
}

#[test]
fn test_accept_offer_disabled_with_approvals() {
    // This test verifies that accepting offers is disabled when sale approvals are enabled
    // We need to:
    // 1. Create marketplace without approvals
    // 2. Create an offer
    // 3. Enable approvals
    // 4. Try to accept offer - should fail
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);

    // Start without approvals to create offer
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let offer_price = coin(1000, "uxion");

    // Create offer (works without approvals)
    let create_offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let create_result = app
        .execute_contract(
            buyer.clone(),
            marketplace_contract.clone(),
            &create_offer_msg,
            std::slice::from_ref(&offer_price),
        )
        .unwrap();

    // Extract offer ID from events
    let offer_id = create_result
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Enable approvals via UpdateConfig
    let update_config_msg = ExecuteMsg::UpdateConfig {
        config: xion_nft_marketplace::state::Config {
            manager: manager.to_string(),
            fee_recipient: manager.to_string(),
            sale_approvals: true, // Enable approvals
            fee_bps: 250,
            listing_denom: "uxion".to_string(),
        },
    };

    app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &update_config_msg,
        &[],
    )
    .unwrap();

    // Query the config to verify approvals are enabled
    let query_msg = xion_nft_marketplace::msg::QueryMsg::Config {};
    let config: xion_nft_marketplace::state::Config<cosmwasm_std::Addr> = app
        .wrap()
        .query_wasm_smart(marketplace_contract.clone(), &query_msg)
        .unwrap();
    assert!(config.sale_approvals, "Sale approvals should be enabled");

    // Approve marketplace to manage NFT
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Try to accept offer - should fail
    let accept_offer_msg = ExecuteMsg::AcceptOffer {
        id: offer_id,
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_offer_msg,
        &[],
    );

    // Should fail with OfferesDisabled error
    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::OfferesDisabled {}.to_string(),
    );
}

#[test]
fn test_create_collection_offer_disabled_with_approvals() {
    // This test verifies that creating collection offers is disabled when sale approvals are enabled
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_with_approvals(&mut app, &manager);

    let offer_price = coin(1000, "uxion");

    // Try to create collection offer - should fail
    let create_collection_offer_msg = ExecuteMsg::CreateCollectionOffer {
        collection: asset_contract.to_string(),
        price: offer_price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &create_collection_offer_msg,
        std::slice::from_ref(&offer_price),
    );

    // Should fail with OfferesDisabled error
    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::OfferesDisabled {}.to_string(),
    );
}

#[test]
fn test_accept_collection_offer_disabled_with_approvals() {
    // This test verifies that accepting collection offers is disabled when sale approvals are enabled
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);

    // Start without approvals to create collection offer
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let offer_price = coin(1000, "uxion");

    // Create collection offer (works without approvals)
    let create_collection_offer_msg = ExecuteMsg::CreateCollectionOffer {
        collection: asset_contract.to_string(),
        price: offer_price.clone(),
    };

    let create_result = app
        .execute_contract(
            buyer.clone(),
            marketplace_contract.clone(),
            &create_collection_offer_msg,
            std::slice::from_ref(&offer_price),
        )
        .unwrap();

    // Extract collection offer ID from events
    let collection_offer_id = create_result
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-collection-offer")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // Enable approvals via UpdateConfig
    let update_config_msg = ExecuteMsg::UpdateConfig {
        config: xion_nft_marketplace::state::Config {
            manager: manager.to_string(),
            fee_recipient: manager.to_string(),
            sale_approvals: true, // Enable approvals
            fee_bps: 250,
            listing_denom: "uxion".to_string(),
        },
    };

    app.execute_contract(
        manager.clone(),
        marketplace_contract.clone(),
        &update_config_msg,
        &[],
    )
    .unwrap();

    // Approve marketplace to manage NFT
    let approve_msg = cw721_base::msg::ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // Try to accept collection offer - should fail
    let accept_collection_offer_msg = ExecuteMsg::AcceptCollectionOffer {
        id: collection_offer_id,
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: offer_price.clone(),
    };

    let result = app.execute_contract(
        seller.clone(),
        marketplace_contract.clone(),
        &accept_collection_offer_msg,
        &[],
    );

    // Should fail with OfferesDisabled error
    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::OfferesDisabled {}.to_string(),
    );
}

use crate::tests::test_helpers::*;
use cosmwasm_std::{coin, Uint128};
use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
use cw_multi_test::Executor;
use xion_nft_marketplace::helpers::query_listing;
use xion_nft_marketplace::msg::{ExecuteMsg, InstantiateMsg};

#[test]
fn test_buy_item_success() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    let price = coin(100, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price: price.clone(),
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());

    let events = result.unwrap().events;
    let listing_id = extract_listing_id_from_events(&events);

    let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
    assert!(listing_resp.is_ok());
    let listing = listing_resp.unwrap();
    assert_eq!(listing.price.amount.u128(), 97);

    use cw_multi_test::{BankSudo, SudoMsg};
    let funds = vec![coin(10000, "uxion")];
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: buyer.to_string(),
        amount: funds,
    }))
    .unwrap();

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

    match result {
        Ok(response) => {
            let events = response.events;
            let sell_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/item-sold");
            assert!(sell_event.is_some());

            let owner_query = cw721_base::msg::QueryMsg::OwnerOf {
                token_id: "token1".to_string(),
                include_expired: Some(false),
            };
            let owner_resp: cw721::msg::OwnerOfResponse = app
                .wrap()
                .query_wasm_smart(asset_contract.clone(), &owner_query)
                .unwrap();
            assert_eq!(owner_resp.owner, buyer.to_string());

            let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
            assert!(listing_resp.is_err());
        }
        Err(error) => {
            println!("Error: {:?}", error);
            panic!("Buy item failed: {:?}", error);
        }
    }
}

#[test]
fn test_buy_item_insufficient_funds() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_for_buy_test(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let insufficient_price = coin(50, "uxion");
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[insufficient_price],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidPayment {
            expected: price,
            actual: coin(50, "uxion"),
        }
        .to_string(),
    );
}

#[test]
fn test_buy_item_wrong_price() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let listing_price = coin(100, "uxion");
    let listing_id = create_listing_for_buy_test(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        listing_price.clone(),
    );

    let wrong_price = coin(150, "uxion");
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: wrong_price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[wrong_price],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidPrice {
            expected: listing_price,
            actual: coin(150, "uxion"),
        }
        .to_string(),
    );
}

#[test]
fn test_buy_item_wrong_denomination() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    let price = coin(100, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price: price.clone(),
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());

    let events = result.unwrap().events;
    let listing_id = extract_listing_id_from_events(&events);

    use cw_multi_test::{BankSudo, SudoMsg};
    let funds = vec![coin(10000, "fakexion")];
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: buyer.to_string(),
        amount: funds,
    }))
    .unwrap();

    let wrong_denom_price = coin(100, "fakexion");
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: wrong_denom_price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[wrong_denom_price],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidPrice {
            expected: coin(100, "uxion"),
            actual: coin(100, "fakexion"),
        }
        .to_string(),
    );
}

#[test]
fn test_buy_item_nonexistent_listing() {
    let mut app = setup_app_with_balances();
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    let price = coin(100, "uxion");
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: "nonexistent-listing-id".to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price],
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Error executing") || error_msg.contains("WasmMsg"));
}

#[test]
fn test_buy_item_self_purchase() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_for_buy_test(
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
        seller.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price],
    );

    assert!(result.is_ok());

    let events = result.unwrap().events;
    let sell_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/item-sold");
    assert!(sell_event.is_some());
}

#[test]
fn test_buy_item_multiple_coins() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_for_buy_test(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    let extra_coin = coin(50, "fakexion");
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        &[price, extra_coin],
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Error executing") || error_msg.contains("WasmMsg"));
}

#[test]
fn test_buy_item_no_coins() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_for_buy_test(
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

    let result = app.execute_contract(buyer.clone(), marketplace_contract.clone(), &buy_msg, &[]);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Error executing") || error_msg.contains("WasmMsg"));
}

#[test]
fn test_buy_item_success_with_royalties() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let royalty_recipient = app.api().addr_make("royalty_recipient");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    let royalty_plugin = asset::plugin::Plugin::Royalty {
        bps: 500,
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

    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    let price = coin(1000, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price: price.clone(),
        token_id: "token1".to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());

    let events = result.unwrap().events;
    let listing_id = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let buyer_balance_before = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let royalty_recipient_balance_before = app
        .wrap()
        .query_balance(&royalty_recipient, "uxion")
        .unwrap()
        .amount;

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

    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    let royalty_recipient_balance_after = app
        .wrap()
        .query_balance(&royalty_recipient, "uxion")
        .unwrap()
        .amount;

    let expected_marketplace_fee = 25u128;
    let expected_royalty = 48u128;
    // (1000 - 25 ) * 5%  = 927
    let expected_seller_payment = 927u128;

    assert_eq!(
        buyer_balance_after,
        buyer_balance_before - Uint128::from(1000u128),
        "Buyer should have paid 1000 uxion"
    );
    assert_eq!(
        seller_balance_after.u128(),
        seller_balance_before.u128() + expected_seller_payment,
        "Seller should receive {} uxion (1000 - 25 marketplace fee - 50 royalty)",
        expected_seller_payment
    );

    assert_eq!(
        manager_balance_after,
        manager_balance_before + Uint128::from(expected_marketplace_fee),
        "Manager should receive {} uxion marketplace fee",
        expected_marketplace_fee
    );

    assert_eq!(
        royalty_recipient_balance_after,
        royalty_recipient_balance_before + Uint128::from(expected_royalty),
        "Royalty recipient should receive {} uxion royalty",
        expected_royalty
    );

    let owner_query = cw721_base::msg::QueryMsg::OwnerOf {
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
fn test_buy_reserved_for() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let reserved_buyer = app.api().addr_make("reserved_buyer");
    let uninterested_buyer = app.api().addr_make("uninterested_buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    // Mint NFT to seller
    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    // Approve marketplace contract
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    // List with reserved_for for reserved_buyer
    let price = coin(100, "uxion");
    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price: price.clone(),
        token_id: "token1".to_string(),
        reserved_for: Some(reserved_buyer.to_string()),
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(
        result.is_ok(),
        "Listing with reserved_for should succeed: {:?}",
        result.err()
    );

    let events = result.unwrap().events;
    let listing_id = extract_listing_id_from_events(&events);

    // Fund both buyers
    use cw_multi_test::{BankSudo, SudoMsg};
    let funds = vec![coin(10000, "uxion")];
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: reserved_buyer.to_string(),
        amount: funds.clone(),
    }))
    .unwrap();
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: uninterested_buyer.to_string(),
        amount: funds,
    }))
    .unwrap();

    // Buyer C (uninterested_buyer) tries to buy and should get error
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };
    let result = app.execute_contract(
        uninterested_buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );
    assert!(
        result.is_err(),
        "Unreserved buyer should not be able to purchase, but got Ok"
    );
    // Optionally check specific error message
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "item is reserved for another address".to_string(),
        }
        .to_string(),
    );

    // Buyer B (reserved_buyer) buys and should succeed
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id,
        price: price.clone(),
    };
    let result = app.execute_contract(
        reserved_buyer.clone(),
        marketplace_contract.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );
    assert!(
        result.is_ok(),
        "Reserved buyer should be able to purchase: {:?}",
        result.err()
    );

    let events = result.unwrap().events;
    let sell_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/item-sold");
    assert!(sell_event.is_some());

    // Confirm ownership transferred to reserved_buyer
    let owner_query = cw721_base::msg::QueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, reserved_buyer.to_string());
}

#[test]
fn test_buy_item_with_zero_marketplace_fee() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);

    // Create marketplace with zero fee and approvals disabled (direct buy path)
    let marketplace_code_id = app.store_code(marketplace_contract());
    let config_json = serde_json::json!({
        "manager": manager.to_string(),
        "fee_recipient": manager.to_string(),
        "sale_approvals": false,
        "fee_bps": 0,
        "listing_denom": "uxion"
    });
    let instantiate_msg = InstantiateMsg {
        config: serde_json::from_value(config_json).unwrap(),
    };
    let marketplace_addr = app
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

    let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    let manager_balance_before = app.wrap().query_balance(&manager, "uxion").unwrap().amount;

    let listing_id = create_listing_helper(
        &mut app,
        &marketplace_addr,
        &asset_contract,
        &seller,
        "token1",
        price.clone(),
    );

    // Direct buy (no approval flow)
    let buy_msg = ExecuteMsg::BuyItem {
        listing_id,
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_addr.clone(),
        &buy_msg,
        std::slice::from_ref(&price),
    );

    assert!(
        result.is_ok(),
        "Direct buy with zero fee should succeed: {:?}",
        result.err()
    );

    // Seller receives full price (no fee deducted)
    let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
    assert_eq!(
        seller_balance_after,
        seller_balance_before + price.amount,
        "Seller should receive full price with zero fee"
    );

    // Manager receives nothing
    let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
    assert_eq!(
        manager_balance_after, manager_balance_before,
        "Manager should not receive any fee"
    );

    // NFT transferred to buyer
    let owner_query = cw721_base::msg::QueryMsg::OwnerOf {
        token_id: "token1".to_string(),
        include_expired: Some(false),
    };
    let owner_resp: cw721::msg::OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(asset_contract.clone(), &owner_query)
        .unwrap();
    assert_eq!(owner_resp.owner, buyer.to_string());
}

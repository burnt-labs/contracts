//! Regression tests for the phase-1 safeguards: non-payable handlers, the minimum listing
//! price floor, and best-effort asset delist cleanup that can never block a refund.

use crate::tests::test_helpers::*;
use cosmwasm_std::{coin, Coin};
use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
use cw_multi_test::Executor;
use xion_nft_marketplace::helpers::query_listing;
use xion_nft_marketplace::msg::{ExecuteMsg, QueryMsg};
use xion_nft_marketplace::state::{Config, Listing};

fn balance(app: &cw_multi_test::App, addr: &cosmwasm_std::Addr) -> u128 {
    app.wrap()
        .query_balance(addr, "uxion")
        .unwrap()
        .amount
        .u128()
}

fn revoke_marketplace_approval(
    app: &mut cw_multi_test::App,
    asset: &cosmwasm_std::Addr,
    marketplace: &cosmwasm_std::Addr,
    seller: &cosmwasm_std::Addr,
    token_id: &str,
) {
    let revoke = Cw721ExecuteMsg::Revoke {
        spender: marketplace.to_string(),
        token_id: token_id.to_string(),
    };
    app.execute_contract(seller.clone(), asset.clone(), &revoke, &[])
        .unwrap();
}

#[test]
fn non_payable_messages_reject_funds() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    let asset = setup_asset_contract(&mut app, &minter);
    let marketplace = setup_marketplace_contract(&mut app, &manager);
    mint_nft(&mut app, &asset, &minter, &seller, "token1");

    let list = ExecuteMsg::ListItem {
        collection: asset.to_string(),
        price: coin(100, "uxion"),
        token_id: "token1".to_string(),
        reserved_for: None,
    };
    let err = app
        .execute_contract(
            seller.clone(),
            marketplace.clone(),
            &list,
            &[coin(1, "uxion")],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("accept funds"),
        "got: {err:#}"
    );

    let cancel = ExecuteMsg::CancelListing {
        listing_id: "whatever".to_string(),
    };
    let err = app
        .execute_contract(
            seller.clone(),
            marketplace.clone(),
            &cancel,
            &[coin(1, "uxion")],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("accept funds"));

    let update = ExecuteMsg::UpdateConfig {
        config: Config::<String> {
            manager: manager.to_string(),
            fee_recipient: manager.to_string(),
            sale_approvals: false,
            fee_bps: 250,
            listing_denom: "uxion".to_string(),
            min_listing_price: None,
        },
    };
    let err = app
        .execute_contract(
            manager.clone(),
            marketplace.clone(),
            &update,
            &[coin(1, "uxion")],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("accept funds"));

    // seller balance untouched by the rejected attempts
    assert_eq!(balance(&app, &seller), 10_000);
}

#[test]
fn minimum_listing_price_floors_every_sale_path() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset = setup_asset_contract(&mut app, &minter);
    let marketplace =
        setup_marketplace_with_config(&mut app, &manager, false, Some(coin(50, "uxion")));
    mint_nft(&mut app, &asset, &minter, &seller, "token1");

    let config: Config<cosmwasm_std::Addr> = app
        .wrap()
        .query_wasm_smart(marketplace.clone(), &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.min_listing_price, Some(coin(50, "uxion")));

    // listing below the floor is rejected
    let approve = Cw721ExecuteMsg::Approve {
        spender: marketplace.to_string(),
        token_id: "token1".to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset.clone(), &approve, &[])
        .unwrap();
    let list_low = ExecuteMsg::ListItem {
        collection: asset.to_string(),
        price: coin(49, "uxion"),
        token_id: "token1".to_string(),
        reserved_for: None,
    };
    let err = app
        .execute_contract(seller.clone(), marketplace.clone(), &list_low, &[])
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("below minimum"),
        "{err:#}"
    );

    // exactly the floor is fine
    let list_ok = ExecuteMsg::ListItem {
        collection: asset.to_string(),
        price: coin(50, "uxion"),
        token_id: "token1".to_string(),
        reserved_for: None,
    };
    app.execute_contract(seller.clone(), marketplace.clone(), &list_ok, &[])
        .unwrap();

    // an offer below the floor can be created (buyer's money) but not accepted
    mint_nft(&mut app, &asset, &minter, &seller, "token2");
    let offer_id = create_offer_helper(
        &mut app,
        &marketplace,
        &asset,
        &buyer,
        "token2",
        coin(40, "uxion"),
    );
    let accept = ExecuteMsg::AcceptOffer {
        id: offer_id,
        collection: asset.to_string(),
        token_id: "token2".to_string(),
        price: coin(40, "uxion"),
    };
    let err = app
        .execute_contract(seller.clone(), marketplace.clone(), &accept, &[])
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("below minimum"),
        "{err:#}"
    );

    // a collection offer below the floor cannot be accepted either
    let coffer = ExecuteMsg::CreateCollectionOffer {
        collection: asset.to_string(),
        price: coin(40, "uxion"),
    };
    let res = app
        .execute_contract(
            buyer.clone(),
            marketplace.clone(),
            &coffer,
            &[coin(40, "uxion")],
        )
        .unwrap();
    let coffer_id = res
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
    let accept_c = ExecuteMsg::AcceptCollectionOffer {
        id: coffer_id,
        collection: asset.to_string(),
        token_id: "token2".to_string(),
        price: coin(40, "uxion"),
    };
    let err = app
        .execute_contract(seller.clone(), marketplace.clone(), &accept_c, &[])
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("below minimum"),
        "{err:#}"
    );

    // a floor in the wrong denom is rejected at config time
    let update = ExecuteMsg::UpdateConfig {
        config: Config::<String> {
            manager: manager.to_string(),
            fee_recipient: manager.to_string(),
            sale_approvals: false,
            fee_bps: 250,
            listing_denom: "uxion".to_string(),
            min_listing_price: Some(Coin::new(1_u128, "uatom")),
        },
    };
    let err = app
        .execute_contract(manager.clone(), marketplace.clone(), &update, &[])
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Invalid listing denom"),
        "{err:#}"
    );
}

#[test]
fn reject_sale_refunds_buyer_even_if_asset_delist_fails() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset = setup_asset_contract(&mut app, &minter);
    let marketplace = setup_marketplace_with_approvals(&mut app, &manager);
    mint_nft(&mut app, &asset, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace,
        &asset,
        &seller,
        "token1",
        price.clone(),
    );

    let buy = ExecuteMsg::BuyItem {
        listing_id: listing_id.clone(),
        price: price.clone(),
    };
    let res = app
        .execute_contract(
            buyer.clone(),
            marketplace.clone(),
            &buy,
            std::slice::from_ref(&price),
        )
        .unwrap();
    let pending_sale_id = res
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
    assert_eq!(balance(&app, &buyer), 9_900);

    // the seller revokes the marketplace's approval, so the asset `Delist` will fail
    revoke_marketplace_approval(&mut app, &asset, &marketplace, &seller, "token1");

    let reject = ExecuteMsg::RejectSale {
        id: pending_sale_id,
    };
    let res = app
        .execute_contract(manager.clone(), marketplace.clone(), &reject, &[])
        .unwrap();

    // refund went through despite the failed cleanup
    assert_eq!(balance(&app, &buyer), 10_000);
    assert!(res.events.iter().any(|e| e
        .attributes
        .iter()
        .any(|a| a.value == "delist_cleanup_failed")));
    // marketplace listing is gone
    assert!(app
        .wrap()
        .query_wasm_smart::<Listing>(marketplace.clone(), &QueryMsg::Listing { listing_id })
        .is_err());
    // the asset-side listing survived (nobody could delist it); the owner can clear it
    assert!(query_listing(&app.wrap(), &asset, "token1").is_ok());
    let delist = asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        asset::msg::AssetExtensionExecuteMsg,
    >::UpdateExtension {
        msg: asset::msg::AssetExtensionExecuteMsg::Delist {
            token_id: "token1".to_string(),
        },
    };
    app.execute_contract(seller.clone(), asset.clone(), &delist, &[])
        .unwrap();
    assert!(query_listing(&app.wrap(), &asset, "token1").is_err());
}

#[test]
fn cancel_listing_succeeds_even_if_asset_delist_fails() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let manager = app.api().addr_make("manager");

    let asset = setup_asset_contract(&mut app, &minter);
    let marketplace = setup_marketplace_contract(&mut app, &manager);
    mint_nft(&mut app, &asset, &minter, &seller, "token1");

    let listing_id = create_listing_helper(
        &mut app,
        &marketplace,
        &asset,
        &seller,
        "token1",
        coin(100, "uxion"),
    );
    revoke_marketplace_approval(&mut app, &asset, &marketplace, &seller, "token1");

    let cancel = ExecuteMsg::CancelListing {
        listing_id: listing_id.clone(),
    };
    let res = app
        .execute_contract(seller.clone(), marketplace.clone(), &cancel, &[])
        .unwrap();
    assert!(res.events.iter().any(|e| e
        .attributes
        .iter()
        .any(|a| a.value == "delist_cleanup_failed")));
    assert!(app
        .wrap()
        .query_wasm_smart::<Listing>(marketplace.clone(), &QueryMsg::Listing { listing_id })
        .is_err());
}

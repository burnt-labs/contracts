//! `AllowedMarketplaces`: restricting which addresses may settle a sale.
//!
//! This contract is a trading venue in its own right -- it holds listings and exposes a public
//! `Buy` -- so a collection can trade without depending on any marketplace. A marketplace that
//! charges a fee records the gross price the buyer pays and lists here at the *net* price, so a
//! routed sale pays the seller the net amount while the fee settles separately.
//!
//! By default both routes are open: a buyer may call `Buy` here directly at the listed net
//! price, or go through the marketplace and pay the gross price. The `AllowedMarketplaces`
//! plugin closes the first route by restricting *who may call* `Buy`, which is how a creator
//! builds a collection that settles only through chosen venues.
//!
//! These tests cover both configurations end to end, since the plugin gates the caller rather
//! than the recipient and only a real cross-contract call shows the marketplace settling while
//! the end buyer arrives as `recipient`.

use crate::tests::test_helpers::*;
use asset::plugin::Plugin;
use cosmwasm_std::{coin, Addr, Uint128};
use cw721::msg::OwnerOfResponse;
use cw721_base::msg::QueryMsg as Cw721QueryMsg;
use cw_multi_test::{App, Executor};
use xion_nft_marketplace::helpers::query_listing;
use xion_nft_marketplace::msg::ExecuteMsg;

type AssetExecMsg = asset::msg::ExecuteMsg<
    cw721::DefaultOptionalNftExtensionMsg,
    cw721::DefaultOptionalCollectionExtensionMsg,
    asset::msg::AssetExtensionExecuteMsg,
>;

/// 250 bps on 100 uxion, rounded up, leaves 97 on the collection.
const GROSS: u128 = 100;
const NET: u128 = 97;
const FEE: u128 = 3;

fn direct_buy_msg(token_id: &str) -> AssetExecMsg {
    AssetExecMsg::UpdateExtension {
        msg: asset::msg::AssetExtensionExecuteMsg::Buy {
            token_id: token_id.to_string(),
            recipient: None,
        },
    }
}

fn set_allowed_marketplaces(app: &mut App, asset: &Addr, creator: &Addr, allowed: Vec<String>) {
    let msg = AssetExecMsg::UpdateExtension {
        msg: asset::msg::AssetExtensionExecuteMsg::SetCollectionPlugin {
            plugins: vec![Plugin::AllowedMarketplaces {
                marketplaces: allowed.iter().map(Addr::unchecked).collect(),
            }],
        },
    };
    app.execute_contract(creator.clone(), asset.clone(), &msg, &[])
        .unwrap();
}

fn balance(app: &App, who: &Addr) -> Uint128 {
    app.wrap().query_balance(who, "uxion").unwrap().amount
}

fn owner_of(app: &App, asset: &Addr, token_id: &str) -> String {
    app.wrap()
        .query_wasm_smart::<OwnerOfResponse>(
            asset.clone(),
            &Cw721QueryMsg::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: Some(false),
            },
        )
        .unwrap()
        .owner
}

/// Default configuration: both routes are open.
///
/// With no restriction set, the net-price listing this contract holds is fillable by anyone.
/// The seller receives the net amount either way; a buyer who settles here directly pays the
/// net price rather than the marketplace's gross price, so no fee is collected.
#[test]
fn a_collection_with_no_restriction_allows_settling_here_directly() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
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
        coin(GROSS, "uxion"),
    );

    // The marketplace publishes the net price on the collection.
    let asset_listing = query_listing(&app.wrap(), &asset, "token1").unwrap();
    assert_eq!(asset_listing.price.amount.u128(), NET);

    let fee_before = balance(&app, &manager);
    let seller_before = balance(&app, &seller);

    app.execute_contract(
        buyer.clone(),
        asset.clone(),
        &direct_buy_msg("token1"),
        &[coin(NET, "uxion")],
    )
    .expect("an unconfigured collection accepts a direct buy");

    assert_eq!(owner_of(&app, &asset, "token1"), buyer.to_string());
    assert_eq!(
        balance(&app, &seller),
        seller_before + Uint128::from(NET),
        "the seller is paid the net amount, the same as through a routed sale"
    );
    assert_eq!(
        balance(&app, &manager),
        fee_before,
        "no fee is collected when the sale does not go through the marketplace"
    );

    // The marketplace's own record now points at a listing that no longer exists.
    assert!(
        app.execute_contract(
            buyer,
            marketplace,
            &ExecuteMsg::BuyItem {
                listing_id,
                price: coin(GROSS, "uxion"),
            },
            &[coin(GROSS, "uxion")],
        )
        .is_err(),
        "the marketplace record no longer resolves once the listing is filled here"
    );
}

/// Restricted configuration: only the named venue may settle.
///
/// `AllowedMarketplaces` gates the *caller* of `Buy`, not the recipient. Naming the
/// marketplace therefore refuses a buyer settling here directly while leaving the routed sale
/// intact, with the end buyer arriving as `recipient`.
#[test]
fn allowed_marketplaces_restricts_settlement_to_the_named_venue() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset = setup_asset_contract(&mut app, &minter);
    let marketplace = setup_marketplace_contract(&mut app, &manager);
    mint_nft(&mut app, &asset, &minter, &seller, "token1");

    // The collection creator -- the minter here -- names the only venue that may settle.
    set_allowed_marketplaces(&mut app, &asset, &minter, vec![marketplace.to_string()]);

    let listing_id = create_listing_helper(
        &mut app,
        &marketplace,
        &asset,
        &seller,
        "token1",
        coin(GROSS, "uxion"),
    );
    assert_eq!(
        query_listing(&app.wrap(), &asset, "token1")
            .unwrap()
            .price
            .amount
            .u128(),
        NET,
        "the net-price listing still exists; it is simply no longer fillable by anyone"
    );

    let fee_before = balance(&app, &manager);
    let seller_before = balance(&app, &seller);

    // Settling here directly is refused.
    let err = app
        .execute_contract(
            buyer.clone(),
            asset.clone(),
            &direct_buy_msg("token1"),
            &[coin(NET, "uxion")],
        )
        .expect_err("settling here directly must be refused once a venue is named");
    assert!(
        err.root_cause()
            .to_string()
            .contains("buyer is not an allowed marketplace"),
        "{err:#}"
    );
    assert_eq!(
        owner_of(&app, &asset, "token1"),
        seller.to_string(),
        "the token has not moved"
    );
    assert_eq!(balance(&app, &seller), seller_before);

    // The routed sale still works, and the fee lands.
    app.execute_contract(
        buyer.clone(),
        marketplace,
        &ExecuteMsg::BuyItem {
            listing_id,
            price: coin(GROSS, "uxion"),
        },
        &[coin(GROSS, "uxion")],
    )
    .expect("the named marketplace may still settle");

    assert_eq!(
        owner_of(&app, &asset, "token1"),
        buyer.to_string(),
        "the end buyer receives the token as `recipient`, not as the caller"
    );
    assert_eq!(balance(&app, &seller), seller_before + Uint128::from(NET));
    assert_eq!(
        balance(&app, &manager),
        fee_before + Uint128::from(FEE),
        "the fee recipient is paid"
    );
}

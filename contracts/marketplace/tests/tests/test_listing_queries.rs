use cosmwasm_std::{coin, testing::mock_dependencies, Addr};
use cw_multi_test::{App, Executor};

use xion_nft_marketplace::{
    msg::{ExecuteMsg, QueryMsg},
    query::{query_listings, query_listings_by_seller},
    state::{listings, Listing, ListingStatus},
};

use super::test_helpers::{
    create_listing, create_listing_helper, mint_nft, setup_app, setup_app_with_balances,
    setup_asset_contract, setup_marketplace_contract, setup_marketplace_with_approvals,
};

struct ListingFixture {
    app: App,
    marketplace: Addr,
    seller: Addr,
    other_seller: Addr,
    listing_ids: Vec<String>,
    seller_listing_ids: Vec<String>,
}

fn setup_listings() -> ListingFixture {
    let mut app = setup_app();
    let manager = app.api().addr_make("manager");
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let other_seller = app.api().addr_make("other-seller");
    let asset = setup_asset_contract(&mut app, &minter);
    let marketplace = setup_marketplace_contract(&mut app, &manager);

    let mut listing_ids = Vec::new();
    let mut seller_listing_ids = Vec::new();
    for (token_id, owner, amount) in [
        ("token-1", seller.clone(), 100),
        ("token-2", seller.clone(), 200),
        ("token-3", other_seller.clone(), 300),
    ] {
        mint_nft(&mut app, &asset, &minter, &owner, token_id);
        let listing_id = create_listing(
            &mut app,
            &marketplace,
            &asset,
            &owner,
            token_id,
            coin(amount, "uxion"),
        );
        if owner == seller {
            seller_listing_ids.push(listing_id.clone());
        }
        listing_ids.push(listing_id);
    }
    listing_ids.sort();
    seller_listing_ids.sort();

    ListingFixture {
        app,
        marketplace,
        seller,
        other_seller,
        listing_ids,
        seller_listing_ids,
    }
}

#[test]
fn listings_supports_pagination() {
    let fixture = setup_listings();

    let first_page: Vec<Listing> = fixture
        .app
        .wrap()
        .query_wasm_smart(
            &fixture.marketplace,
            &QueryMsg::Listings {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();
    assert_eq!(
        first_page
            .iter()
            .map(|listing| &listing.id)
            .collect::<Vec<_>>(),
        fixture.listing_ids.iter().take(2).collect::<Vec<_>>()
    );
    let second_page: Vec<Listing> = fixture
        .app
        .wrap()
        .query_wasm_smart(
            &fixture.marketplace,
            &QueryMsg::Listings {
                start_after: Some(first_page.last().unwrap().id.clone()),
                limit: Some(2),
            },
        )
        .unwrap();
    assert_eq!(second_page.len(), 1);
    assert_eq!(second_page[0].id, fixture.listing_ids[2]);

    let non_existent_cursor = format!("{}0", fixture.listing_ids[0]);
    assert!(!fixture.listing_ids.contains(&non_existent_cursor));
    let after_non_existent_cursor: Vec<Listing> = fixture
        .app
        .wrap()
        .query_wasm_smart(
            &fixture.marketplace,
            &QueryMsg::Listings {
                start_after: Some(non_existent_cursor),
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        after_non_existent_cursor
            .iter()
            .map(|listing| &listing.id)
            .collect::<Vec<_>>(),
        fixture.listing_ids.iter().skip(1).collect::<Vec<_>>()
    );
}

#[test]
fn listings_by_seller_filters_and_paginates() {
    let fixture = setup_listings();

    let first_page: Vec<Listing> = fixture
        .app
        .wrap()
        .query_wasm_smart(
            &fixture.marketplace,
            &QueryMsg::ListingsBySeller {
                seller: fixture.seller.to_string(),
                start_after: None,
                limit: Some(1),
            },
        )
        .unwrap();
    assert_eq!(first_page.len(), 1);
    assert_eq!(first_page[0].seller, fixture.seller);
    assert_eq!(first_page[0].id, fixture.seller_listing_ids[0]);

    let second_page: Vec<Listing> = fixture
        .app
        .wrap()
        .query_wasm_smart(
            &fixture.marketplace,
            &QueryMsg::ListingsBySeller {
                seller: fixture.seller.to_string(),
                start_after: Some(first_page[0].id.clone()),
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(second_page.len(), 1);
    assert_eq!(second_page[0].seller, fixture.seller);
    assert_eq!(second_page[0].id, fixture.seller_listing_ids[1]);

    let other_seller_listings: Vec<Listing> = fixture
        .app
        .wrap()
        .query_wasm_smart(
            &fixture.marketplace,
            &QueryMsg::ListingsBySeller {
                seller: fixture.other_seller.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(other_seller_listings.len(), 1);
    assert_eq!(other_seller_listings[0].seller, fixture.other_seller);
}

#[test]
fn listing_queries_apply_default_and_max_limits() {
    let mut deps = mock_dependencies();
    let seller = deps.api.addr_make("seller");
    let collection = deps.api.addr_make("collection");

    for index in 0u32..101 {
        let id = format!("listing-{index:03}");
        listings()
            .save(
                deps.as_mut().storage,
                id.clone(),
                &Listing {
                    id,
                    collection: collection.clone(),
                    token_id: format!("token-{index:03}"),
                    price: coin(u128::from(index) + 1, "uxion"),
                    asset_price: coin(u128::from(index) + 1, "uxion"),
                    seller: seller.clone(),
                    reserved_for: None,
                    status: ListingStatus::Active,
                },
            )
            .unwrap();
    }

    let default_page = query_listings(deps.as_ref(), None, None).unwrap();
    assert_eq!(default_page.len(), 50);

    let capped_page = query_listings(deps.as_ref(), None, Some(101)).unwrap();
    assert_eq!(capped_page.len(), 100);

    let capped_seller_page =
        query_listings_by_seller(deps.as_ref(), seller.to_string(), None, Some(101)).unwrap();
    assert_eq!(capped_seller_page.len(), 100);
}

#[test]
fn listings_by_seller_rejects_invalid_address() {
    let mut app = setup_app();
    let manager = app.api().addr_make("manager");
    let marketplace = setup_marketplace_contract(&mut app, &manager);

    let result = app.wrap().query_wasm_smart::<Vec<Listing>>(
        &marketplace,
        &QueryMsg::ListingsBySeller {
            seller: "not a valid address".to_string(),
            start_after: None,
            limit: None,
        },
    );

    assert!(result.is_err());
}

#[test]
fn listing_queries_include_reserved_listings() {
    let mut app = setup_app_with_balances();
    let manager = app.api().addr_make("manager");
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let asset = setup_asset_contract(&mut app, &minter);
    let marketplace = setup_marketplace_with_approvals(&mut app, &manager);
    let price = coin(100, "uxion");

    mint_nft(&mut app, &asset, &minter, &seller, "token-1");
    let listing_id = create_listing_helper(
        &mut app,
        &marketplace,
        &asset,
        &seller,
        "token-1",
        price.clone(),
    );
    app.execute_contract(
        buyer,
        marketplace.clone(),
        &ExecuteMsg::BuyItem {
            listing_id: listing_id.clone(),
            price: price.clone(),
        },
        std::slice::from_ref(&price),
    )
    .unwrap();

    let listings: Vec<Listing> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::Listings {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(listings.len(), 1);
    assert_eq!(listings[0].id, listing_id);
    assert_eq!(listings[0].status, ListingStatus::Reserved);

    let seller_listings: Vec<Listing> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::ListingsBySeller {
                seller: seller.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(seller_listings, listings);
}

#[test]
fn listing_queries_return_empty_results() {
    let mut app = setup_app();
    let manager = app.api().addr_make("manager");
    let seller = app.api().addr_make("seller");
    let marketplace = setup_marketplace_contract(&mut app, &manager);

    let listings: Vec<Listing> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::Listings {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(listings.is_empty());

    let seller_listings: Vec<Listing> = app
        .wrap()
        .query_wasm_smart(
            &marketplace,
            &QueryMsg::ListingsBySeller {
                seller: seller.to_string(),
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert!(seller_listings.is_empty());
}

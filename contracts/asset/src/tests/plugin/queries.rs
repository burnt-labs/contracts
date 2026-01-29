use cosmwasm_std::{
    Coin, Empty, from_json,
    testing::{mock_dependencies, mock_env},
};
use cw721::{state::NftInfo, traits::Cw721Query};

use crate::{
    msg::AssetExtensionQueryMsg, plugin::Plugin, state::ListingInfo, traits::DefaultAssetContract,
};

#[test]
fn get_listing_returns_saved_listing() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let seller = deps.api.addr_make("seller");
    let nft_info = NftInfo {
        owner: seller.clone(),
        approvals: vec![],
        token_uri: None,
        extension: Empty::default(),
    };
    let listing = ListingInfo {
        id: "token-1".to_string(),
        price: Coin::new(100u128, "uxion"),
        seller: seller.clone(),
        reserved: None,
    };

    contract
        .config
        .listings
        .save(deps.as_mut().storage, "token-1", &listing)
        .unwrap();
    contract
        .config
        .cw721_config
        .nft_info
        .save(deps.as_mut().storage, "token-1", &nft_info)
        .unwrap();

    let binary = contract
        .query_extension(
            deps.as_ref(),
            &mock_env(),
            AssetExtensionQueryMsg::GetListing {
                token_id: "token-1".to_string(),
            },
        )
        .unwrap();

    let fetched: ListingInfo = from_json(binary).unwrap();
    assert_eq!(fetched, listing);
}

#[test]
fn get_listings_by_seller_supports_pagination() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let seller = deps.api.addr_make("seller");
    let other = deps.api.addr_make("other");

    for (id, owner) in [
        ("token-1", seller.clone()),
        ("token-2", seller.clone()),
        ("token-3", seller.clone()),
        ("token-4", other.clone()),
    ] {
        let nft_info = NftInfo {
            owner: owner.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty::default(),
        };
        let listing = ListingInfo {
            id: id.to_string(),
            price: Coin::new(50u128, "uxion"),
            seller: owner.clone(),
            reserved: None,
        };
        contract
            .config
            .listings
            .save(deps.as_mut().storage, id, &listing)
            .unwrap();
        contract
            .config
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, id, &nft_info)
            .unwrap();
    }

    let first_page = contract
        .query_extension(
            deps.as_ref(),
            &mock_env(),
            AssetExtensionQueryMsg::GetListingsBySeller {
                seller: deps.api.addr_make("seller").to_string(),
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();
    let first_page: Vec<ListingInfo> = from_json(first_page).unwrap();
    assert_eq!(first_page.len(), 2);
    assert_eq!(first_page[0].id, "token-1");
    assert_eq!(first_page[1].id, "token-2");
    assert!(first_page.iter().all(|listing| listing.seller == seller));

    let second_page = contract
        .query_extension(
            deps.as_ref(),
            &mock_env(),
            AssetExtensionQueryMsg::GetListingsBySeller {
                seller: deps.api.addr_make("seller").to_string(),
                start_after: Some("token-1".to_string()),
                limit: None,
            },
        )
        .unwrap();
    let second_page: Vec<ListingInfo> = from_json(second_page).unwrap();
    assert_eq!(second_page.len(), 2);
    assert_eq!(second_page[0].id, "token-2");
    assert_eq!(second_page[1].id, "token-3");
    assert!(second_page.iter().all(|listing| listing.seller == seller));
}

#[test]
fn get_all_listings_supports_pagination() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let seller = deps.api.addr_make("seller");
    let other = deps.api.addr_make("other");

    for (id, owner) in [
        ("token-1", seller.clone()),
        ("token-2", seller.clone()),
        ("token-3", seller.clone()),
        ("token-4", other.clone()),
    ] {
        let nft_info = NftInfo {
            owner: owner.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty::default(),
        };
        let listing = ListingInfo {
            id: id.to_string(),
            price: Coin::new(50u128, "uxion"),
            seller: owner.clone(),
            reserved: None,
        };
        contract
            .config
            .listings
            .save(deps.as_mut().storage, id, &listing)
            .unwrap();
        contract
            .config
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, id, &nft_info)
            .unwrap();
    }

    let first_page = contract
        .query_extension(
            deps.as_ref(),
            &mock_env(),
            AssetExtensionQueryMsg::GetAllListings {
                start_after: None,
                limit: Some(2),
            },
        )
        .unwrap();
    let first_page: Vec<ListingInfo> = from_json(first_page).unwrap();
    assert_eq!(first_page.len(), 2);
    assert_eq!(first_page[0].id, "token-1");
    assert_eq!(first_page[1].id, "token-2");

    let second_page = contract
        .query_extension(
            deps.as_ref(),
            &mock_env(),
            AssetExtensionQueryMsg::GetAllListings {
                start_after: Some("token-1".to_string()),
                limit: None,
            },
        )
        .unwrap();
    let second_page: Vec<ListingInfo> = from_json(second_page).unwrap();
    assert_eq!(second_page.len(), 3);
    assert_eq!(second_page[0].id, "token-2");
    assert_eq!(second_page[1].id, "token-3");
    assert_eq!(second_page[2].id, "token-4");
}

#[test]
fn get_collection_plugins_returns_all_plugins() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "MinimumPrice",
            &Plugin::MinimumPrice {
                amount: Coin::new(100u128, "uxion"),
            },
        )
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "AllowedCurrencies",
            &Plugin::AllowedCurrencies {
                denoms: vec![Coin::new(0u128, "uxion"), Coin::new(0u128, "uusdc")],
            },
        )
        .unwrap();

    let binary = contract
        .query_extension(
            deps.as_ref(),
            &mock_env(),
            AssetExtensionQueryMsg::GetCollectionPlugins {},
        )
        .unwrap();
    let plugins: Vec<Plugin> = from_json(binary).unwrap();

    assert_eq!(plugins.len(), 2);
    assert!(plugins.contains(&Plugin::MinimumPrice {
        amount: Coin::new(100u128, "uxion"),
    }));
    assert!(plugins.contains(&Plugin::AllowedCurrencies {
        denoms: vec![Coin::new(0u128, "uxion"), Coin::new(0u128, "uusdc")],
    }));
}

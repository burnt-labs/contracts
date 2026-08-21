use cosmwasm_std::{
    Coin, Empty,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::{Approval, Expiration, state::NftInfo};

use crate::{
    error::ContractError,
    execute::delist,
    state::{AssetConfig, ListingInfo, Reserve},
};

use super::helpers::{expect_err, expect_ok};

#[test]
fn delist_flow() {
    // successful delist removes listing and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &nft_info,
        ));
        let price = Coin::new(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: None,
            },
        ));

        let response = expect_ok(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-1".to_string(),
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "delist".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
            ],
        );

        assert!(
            expect_ok(
                AssetConfig::<Empty>::default()
                    .listings
                    .may_load(deps.as_ref().storage, "token-1")
            )
            .is_none()
        );
    }

    // non-admin cannot delist
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let intruder_addr = deps.api.addr_make("intruder");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &nft_info,
        ));
        let price = Coin::new(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-2",
            &ListingInfo {
                id: "token-2".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: None,
            },
        ));
        let err = expect_err(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-2".to_string(),
        ));
        assert_eq!(err, ContractError::Unauthorized {});
    }
    // non-existent listing is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let err = expect_err(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-3".to_string(),
        ));
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-3".to_string()
            }
        );
    }
    // stale listings cannot be delisted
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let original_owner = deps.api.addr_make("original");
        let new_owner = deps.api.addr_make("new_owner");
        let nft_info = NftInfo {
            owner: original_owner.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &nft_info,
        ));
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-4",
            &ListingInfo {
                id: "token-4".to_string(),
                seller: original_owner.clone(),
                price: Coin::new(100_u128, "uxion"),
                reserved: None,
            },
        ));
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &NftInfo {
                owner: new_owner.clone(),
                approvals: vec![],
                token_uri: None,
                extension: Empty {},
            },
        ));

        let err = expect_err(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&new_owner, &[]),
            "token-4".to_string(),
        ));
        assert_eq!(err, ContractError::StaleListing {});
    }
}

#[test]
fn delist_respects_active_reservations() {
    // the seller cannot delist while a third party holds a live reservation
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let reserver_addr = deps.api.addr_make("marketplace");
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &NftInfo {
                owner: seller_addr.clone(),
                // the marketplace holds an approval, as it does in the real flow
                approvals: vec![Approval {
                    spender: reserver_addr.clone(),
                    expires: Expiration::Never {},
                }],
                token_uri: None,
                extension: Empty {},
            },
        ));
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: seller_addr.clone(),
                price: Coin::new(100_u128, "uxion"),
                reserved: Some(Reserve {
                    reserver: reserver_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
            },
        ));

        let err = expect_err(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-1".to_string(),
        ));
        assert_eq!(
            err,
            ContractError::ReservedAsset {
                id: "token-1".to_string()
            }
        );

        // the reserver itself may delist while its reservation is live
        expect_ok(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-1".to_string(),
        ));
    }

    // the seller can delist again once the reservation has expired
    {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let reserver_addr = deps.api.addr_make("marketplace");
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &NftInfo {
                owner: seller_addr.clone(),
                approvals: vec![],
                token_uri: None,
                extension: Empty {},
            },
        ));
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-2",
            &ListingInfo {
                id: "token-2".to_string(),
                seller: seller_addr.clone(),
                price: Coin::new(100_u128, "uxion"),
                reserved: Some(Reserve {
                    reserver: reserver_addr,
                    reserved_until: env.block.time.plus_seconds(600),
                }),
            },
        ));

        env.block.time = env.block.time.plus_seconds(601);
        expect_ok(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-2".to_string(),
        ));
    }
}

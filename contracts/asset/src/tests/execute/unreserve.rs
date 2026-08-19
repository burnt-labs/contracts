use cosmwasm_std::{
    Coin, Empty,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::state::NftInfo;

use crate::{
    error::ContractError,
    execute::unreserve,
    state::{AssetConfig, ListingInfo, Reserve},
};

use super::helpers::{expect_err, expect_ok};

#[test]
fn unreserve_flow() {
    // reserver can remove reservation while keeping listing active
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &nft_info,
        ));

        let reservation = Reserve {
            reserver: reserver_addr.clone(),
            reserved_until: env.block.time.plus_seconds(600),
        };

        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: owner_addr.clone(),
                price: Coin::new(100_u128, "uxion"),
                reserved: Some(reservation.clone()),
            },
        ));

        let response = expect_ok(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-1".to_string(),
            false,
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "unreserve".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("reserver".to_string(), reserver_addr.to_string()),
                ("delisted".to_string(), "false".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-1"),
        );
        assert!(stored.reserved.is_none());
    }

    // reserver can delist when requested
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &nft_info,
        ));

        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-2",
            &ListingInfo {
                id: "token-2".to_string(),
                seller: owner_addr.clone(),
                price: Coin::new(150_u128, "uxion"),
                reserved: Some(Reserve {
                    reserver: reserver_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
            },
        ));

        let response = expect_ok(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-2".to_string(),
            true,
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "unreserve".to_string()),
                ("id".to_string(), "token-2".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("reserver".to_string(), reserver_addr.to_string()),
                ("delisted".to_string(), "true".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .may_load(deps.as_ref().storage, "token-2"),
        );
        assert!(stored.is_none());
    }

    // non-reserver cannot unreserve
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let intruder_addr = deps.api.addr_make("intruder");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-3",
            &nft_info,
        ));

        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-3",
            &ListingInfo {
                id: "token-3".to_string(),
                seller: owner_addr.clone(),
                price: Coin::new(200_u128, "uxion"),
                reserved: Some(Reserve {
                    reserver: reserver_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
            },
        ));

        let err = expect_err(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-3".to_string(),
            false,
        ));
        assert_eq!(err, ContractError::Unauthorized {});
    }

    // cannot unreserve when listing not reserved
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
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
                seller: owner_addr.clone(),
                price: Coin::new(250_u128, "uxion"),
                reserved: None,
            },
        ));

        let err = expect_err(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-4".to_string(),
            false,
        ));
        assert_eq!(
            err,
            ContractError::ReservationNotFound {
                id: "token-4".to_string()
            }
        );
    }
    // stale listings cannot be unreserved
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let original_owner = deps.api.addr_make("original");
        let new_owner = deps.api.addr_make("new_owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let nft_info = NftInfo {
            owner: original_owner.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-5",
            &nft_info,
        ));
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-5",
            &ListingInfo {
                id: "token-5".to_string(),
                seller: original_owner.clone(),
                price: Coin::new(200_u128, "uxion"),
                reserved: Some(Reserve {
                    reserver: reserver_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
            },
        ));
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-5",
            &NftInfo {
                owner: new_owner.clone(),
                approvals: vec![],
                token_uri: None,
                extension: Empty {},
            },
        ));

        let err = expect_err(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-5".to_string(),
            false,
        ));
        assert_eq!(err, ContractError::StaleListing {});
    }
}

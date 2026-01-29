use cosmwasm_std::{
    Coin, Empty,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::state::NftInfo;

use crate::{
    error::ContractError,
    execute::reserve,
    msg::ReserveMsg,
    state::{AssetConfig, ListingInfo},
};

use super::helpers::{expect_err, expect_ok, expect_some};

#[test]
fn reserve_flow() {
    // successful reserve stores state and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let buyer_addr = deps.api.addr_make("buyer");
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

        // cannot reserve unlisted item
        let reservation = ReserveMsg {
            reserver: Some(buyer_addr.clone().to_string()),
            reserved_until: env.block.time.plus_seconds(600),
        };
        let err = expect_err(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            reservation.clone(),
        ));
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-1".to_string()
            }
        );
        // list item first
        let price = Coin::new(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: owner_addr.clone(),
                price: price.clone(),
                reserved: None,
            },
        ));
        let response = expect_ok(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            reservation.clone(),
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "reserve".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("reserver".to_string(), buyer_addr.to_string()),
                (
                    "reserved_until".to_string(),
                    reservation.reserved_until.to_string()
                ),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-1"),
        );
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_some());
        let reserved = expect_some(stored.reserved);
        assert_eq!(reserved.reserver, buyer_addr);
        assert_eq!(reserved.reserved_until, reservation.reserved_until);

        // cannot reserve already reserved item
        let err = expect_err(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            reservation.clone(),
        ));
        assert_eq!(
            err,
            ContractError::ReservedAsset {
                id: "token-1".to_string()
            }
        );
    }
    // reservation must be in the future
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
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
                price: Coin::new(100_u128, "uxion"),
                reserved: None,
            },
        ));
        let reservation = ReserveMsg {
            reserver: None,
            reserved_until: env.block.time,
        };
        let err = expect_err(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-2".to_string(),
            reservation,
        ));
        assert_eq!(
            err,
            ContractError::InvalidReservationExpiration {
                reserved_until: env.block.time.seconds()
            }
        );
    }
    // stale listings are rejected
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
            "token-3",
            &nft_info,
        ));
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-3",
            &ListingInfo {
                id: "token-3".to_string(),
                seller: original_owner.clone(),
                price: Coin::new(100_u128, "uxion"),
                reserved: None,
            },
        ));
        // simulate direct transfer outside contract
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-3",
            &NftInfo {
                owner: new_owner.clone(),
                approvals: vec![],
                token_uri: None,
                extension: Empty {},
            },
        ));

        let err = expect_err(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&new_owner, &[]),
            "token-3".to_string(),
            ReserveMsg {
                reserver: None,
                reserved_until: env.block.time.plus_seconds(10),
            },
        ));
        assert_eq!(err, ContractError::StaleListing {});
    }
}

use cosmwasm_std::{
    Coin, Empty, StdError,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::{Approval, Expiration, state::NftInfo};

use crate::{error::ContractError, execute::list, msg::ReserveMsg, state::AssetConfig};

use super::helpers::{expect_err, expect_ok};

#[test]
fn list_flow() {
    // successful listing stores state and emits attributes
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
            "token-1",
            &nft_info,
        ));

        let price = Coin::new(100_u128, "uxion");
        let response = expect_ok(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            price.clone(),
            None,
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "list".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), owner_addr.to_string()),
                ("reserved_until".to_string(), "none".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-1"),
        );
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_none());

        let duplicate_err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            Coin::new(200_u128, "uxion"),
            None,
        ));
        assert_eq!(
            duplicate_err,
            ContractError::ListingAlreadyExists {
                id: "token-1".to_string(),
            }
        );
    }

    // non-owner cannot list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let nft_info = NftInfo {
            owner: deps.api.addr_make("owner"),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &nft_info,
        ));

        let intruder_addr = deps.api.addr_make("intruder");
        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-2".to_string(),
            Coin::new(100_u128, "uxion"),
            None,
        ));
        assert_eq!(err, ContractError::Unauthorized {});
    }

    // approvals can list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let approver_addr = deps.api.addr_make("approver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![Approval {
                spender: approver_addr.clone(),
                expires: Expiration::AtHeight(env.block.height + 100),
            }],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-3",
            &nft_info,
        ));

        let price = Coin::new(100_u128, "uxion");
        let response = expect_ok(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&approver_addr, &[]),
            "token-3".to_string(),
            price.clone(),
            None,
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "list".to_string()),
                ("id".to_string(), "token-3".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), owner_addr.to_string()),
                ("reserved_until".to_string(), "none".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-3"),
        );
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_none());
    }

    // operators can list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let operator_addr = deps.api.addr_make("operator");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        let asset_config = AssetConfig::<Empty>::default();
        expect_ok(asset_config.cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &nft_info,
        ));
        expect_ok(asset_config.cw721_config.operators.save(
            deps.as_mut().storage,
            (&owner_addr, &operator_addr),
            &Expiration::AtHeight(env.block.height + 100),
        ));

        let price = Coin::new(100_u128, "uxion");
        let response = expect_ok(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&operator_addr, &[]),
            "token-4".to_string(),
            price.clone(),
            None,
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "list".to_string()),
                ("id".to_string(), "token-4".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), owner_addr.to_string()),
                ("reserved_until".to_string(), "none".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-4"),
        );
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_none());
    }

    // expired approvals or operators cannot list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let exp_approval_addr = deps.api.addr_make("bad_actor");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![Approval {
                spender: exp_approval_addr.clone(),
                expires: Expiration::AtHeight(env.block.height - 1),
            }],
            token_uri: None,
            extension: Empty {},
        };
        let asset_config = AssetConfig::<Empty>::default();
        expect_ok(asset_config.cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-5",
            &nft_info,
        ));
        expect_ok(asset_config.cw721_config.operators.save(
            deps.as_mut().storage,
            (&owner_addr, &exp_approval_addr),
            &Expiration::AtHeight(env.block.height - 1),
        ));

        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&exp_approval_addr, &[]),
            "token-5".to_string(),
            Coin::new(100_u128, "uxion"),
            None,
        ));
        assert_eq!(err, ContractError::Unauthorized {});
    }

    // zero-priced listing is rejected
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
            "token-3",
            &nft_info,
        ));

        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-3".to_string(),
            Coin::new(0_u128, "uxion"),
            None,
        ));
        assert_eq!(err, ContractError::InvalidListingPrice { price: 0 });
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
            "token-6",
            &nft_info,
        ));

        let reservation = ReserveMsg {
            reserver: None,
            reserved_until: env.block.time,
        };

        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-6".to_string(),
            Coin::new(100_u128, "uxion"),
            Some(reservation),
        ));
        assert_eq!(
            err,
            ContractError::InvalidReservationExpiration {
                reserved_until: env.block.time.seconds()
            }
        );
    }
    // non-existent item cannot be listed
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");

        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-999".to_string(),
            Coin::new(100_u128, "uxion"),
            None,
        ));
        match err {
            ContractError::Std(StdError::NotFound { .. }) => {}
            _ => panic!("expected NotFound error"),
        }
    }
}

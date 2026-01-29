use cosmwasm_std::{
    BankMsg, CosmosMsg, Empty, coin, coins,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::state::NftInfo;

use crate::{
    error::ContractError,
    execute::buy,
    state::{AssetConfig, ListingInfo, Reserve},
};

use super::helpers::{expect_err, expect_ok};

#[test]
fn buy_flow() {
    // successful buy transfers ownership, pays seller, pays royalties, removes listing, and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let buyer_addr = deps.api.addr_make("buyer");
        let owner_addr = deps.api.addr_make("owner");
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
        let price = coin(100_u128, "uxion");
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

        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, std::slice::from_ref(&price)),
            "token-1".to_string(),
            None,
            vec![(
                owner_addr.to_string(),
                coin(10_u128, "uxion"),
                "royalties".to_string(),
            )],
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "buy".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), price.denom.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
                ("buyer".to_string(), buyer_addr.to_string()),
            ],
        );
        let msgs: Vec<CosmosMsg<Empty>> = response
            .messages
            .iter()
            .map(|msg| msg.msg.clone())
            .collect();
        assert_eq!(
            msgs,
            vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: seller_addr.to_string(),
                amount: coins(90_u128, "uxion"),
            })],
        );

        let stored_nft_info = expect_ok(
            AssetConfig::<Empty>::default()
                .cw721_config
                .nft_info
                .load(deps.as_ref().storage, "token-1"),
        );
        assert_eq!(stored_nft_info.owner, buyer_addr);
        assert!(stored_nft_info.approvals.is_empty());
        assert!(
            expect_ok(
                AssetConfig::<Empty>::default()
                    .listings
                    .may_load(deps.as_ref().storage, "token-1")
            )
            .is_none()
        );
    }
    // insufficient payment is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let buyer_addr = deps.api.addr_make("buyer");
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
        let price = coin(100_u128, "uxion");
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

        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[coin(50_u128, "uxion")]),
            "token-2".to_string(),
            None,
            vec![],
        ));
        assert_eq!(
            err,
            ContractError::InvalidPayment {
                price: 50,
                denom: "uxion".to_string()
            }
        );
    }
    // non-existent listing is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let buyer_addr = deps.api.addr_make("buyer");
        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[coin(100_u128, "uxion")]),
            "token-3".to_string(),
            None,
            vec![],
        ));
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-3".to_string()
            }
        );
    }
    // reserved assets can only be bought by reserver
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let buyer_addr = deps.api.addr_make("buyer");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &nft_info,
        ));
        let price = coin(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-4",
            &ListingInfo {
                id: "token-4".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: Some(Reserve {
                    reserver: buyer_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
            },
        ));

        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, std::slice::from_ref(&price)),
            "token-4".to_string(),
            None,
            vec![],
        ));
        assert_eq!(err, ContractError::Unauthorized {});

        // successful buy by reserver
        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, std::slice::from_ref(&price)),
            "token-4".to_string(),
            None,
            vec![],
        ));
        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "buy".to_string()),
                ("id".to_string(), "token-4".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), price.denom.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
                ("buyer".to_string(), buyer_addr.to_string()),
            ],
        );
        // reset ownership back to seller for subsequent scenario
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &NftInfo {
                owner: seller_addr.clone(),
                approvals: vec![],
                token_uri: None,
                extension: Empty {},
            },
        ));
    }
    // expired reservation is cleared and purchase is allowed
    {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let reserver_addr = deps.api.addr_make("reserver");
        let outsider_addr = deps.api.addr_make("outsider");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-5",
            &nft_info,
        ));
        let price = coin(200_u128, "uxion");
        let reserved_until = env.block.time.plus_seconds(10);
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-5",
            &ListingInfo {
                id: "token-5".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: Some(Reserve {
                    reserver: reserver_addr.clone(),
                    reserved_until,
                }),
            },
        ));

        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&outsider_addr, std::slice::from_ref(&price)),
            "token-5".to_string(),
            None,
            vec![],
        ));
        assert_eq!(err, ContractError::Unauthorized {});

        env.block.time = reserved_until.plus_seconds(1);
        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&outsider_addr, std::slice::from_ref(&price)),
            "token-5".to_string(),
            None,
            vec![],
        ));
        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "buy".to_string()),
                ("id".to_string(), "token-5".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), price.denom.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
                ("buyer".to_string(), outsider_addr.to_string()),
            ],
        );
        assert!(
            expect_ok(
                AssetConfig::<Empty>::default()
                    .listings
                    .may_load(deps.as_ref().storage, "token-5")
            )
            .is_none()
        );
    }
}

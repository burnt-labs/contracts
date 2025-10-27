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
                marketplace_fee_bps: Some(100),
                marketplace_fee_recipient: Some(seller_addr.clone()),
            },
        ));

        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[price.clone()]),
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
                ("marketplace_fee".to_string(), "1".to_string()),
                ("marketplace_fee_recipient".to_string(), seller_addr.clone().to_string()),
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
            vec![
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: seller_addr.to_string(),
                    amount: coins(1_u128, "uxion"),
                }),
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: seller_addr.to_string(),
                    amount: coins(89_u128, "uxion"),
                })
            ],
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
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
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
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[price.clone()]),
            "token-4".to_string(),
            None,
            vec![],
        ));
        assert_eq!(err, ContractError::Unauthorized {});

        // successful buy by reserver
        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[price.clone()]),
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
        // successful buy on behalf of reserver
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
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));
        let mut deps = deps;
        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[price.clone()]),
            "token-4".to_string(),
            Some(buyer_addr.to_string()), // buyer is the reserver
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
    }
}

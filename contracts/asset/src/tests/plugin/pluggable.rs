use std::time::Duration;

use cosmwasm_std::{
    Addr, Binary, Coin, Deps, Empty, Env, MessageInfo, Response, Timestamp,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::{
    Expiration,
    state::{CREATOR, NftInfo},
};

use crate::{
    msg::{AssetExtensionExecuteMsg, ReserveMsg},
    plugin::{DefaultXionAssetContext, Plugin, PluginCtx, RoyaltyInfo},
    state::ListingInfo,
    traits::{AssetContract, DefaultAssetContract, PluggableAsset},
};

fn env_at(time: u64) -> Env {
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(time);
    env
}

fn build_ctx(
    deps: Deps<'_>,
    env: Env,
    info: MessageInfo,
) -> PluginCtx<'_, DefaultXionAssetContext, Empty> {
    PluginCtx {
        deps,
        env,
        info,
        response: Response::default(),
        royalty: RoyaltyInfo::default(),
        data: DefaultXionAssetContext::default(),
        deductions: vec![],
    }
}

#[test]
fn on_list_plugin_runs_all_configured_plugins() {
    let mut deps = mock_dependencies();
    let contract =
        AssetContract::<'_, Empty, Empty, Empty, Empty, AssetExtensionExecuteMsg>::default();

    let min_price = Coin::new(50_u128, "uxion");
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "MinimumPrice",
            &Plugin::MinimumPrice {
                amount: min_price.clone(),
            },
        )
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "NotBefore",
            &Plugin::NotBefore {
                time: Expiration::AtTime(Timestamp::from_seconds(500)),
            },
        )
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "NotAfter",
            &Plugin::NotAfter {
                time: Expiration::AtTime(Timestamp::from_seconds(1_500)),
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

    let env = env_at(1_000);
    let info = message_info(&deps.api.addr_make("seller"), &[]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);
    let token_id = "token-1".to_string();
    let price = Coin::new(100u128, "uxion");

    let result = contract.on_list_plugin(&token_id, &price, &None, &mut ctx);

    assert!(result.unwrap());
    assert_eq!(ctx.data.token_id, token_id);
    assert_eq!(ctx.data.ask_price, Some(price));
    assert_eq!(ctx.data.min_price, Some(min_price));
    assert_eq!(ctx.data.marketplace_fee_bps, None);
    let allowed = ctx.data.allowed_currencies.expect("allowed currencies");
    assert!(allowed.iter().any(|coin| coin.denom == "uxion"));
}

#[test]
fn on_list_plugin_returns_error_when_not_after_fails() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "MinimumPrice",
            &Plugin::MinimumPrice {
                amount: Coin::new(10u128, "uxion"),
            },
        )
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "NotAfter",
            &Plugin::NotAfter {
                time: Expiration::AtTime(Timestamp::from_seconds(500)),
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&deps.api.addr_make("seller"), &[]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);
    let token_id = "token-1".to_string();
    let price = Coin::new(100u128, "uxion");

    let result = contract.on_list_plugin(&token_id, &price, &None, &mut ctx);

    assert_eq!(
        result.expect_err("expected not after error").to_string(),
        cosmwasm_std::StdError::generic_err(format!(
            "Current time {} is after the allowed listing time {}",
            ctx.env.block.time, ctx.data.not_after
        ))
        .to_string()
    );
}

#[test]
fn on_list_plugin_returns_error_when_min_price_fails() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "MinimumPrice",
            &Plugin::MinimumPrice {
                amount: Coin::new(150u128, "uxion"),
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&deps.api.addr_make("seller"), &[]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);
    let token_id = "token-1".to_string();
    let price = Coin::new(100u128, "uxion");

    let result = contract.on_list_plugin(&token_id, &price, &None, &mut ctx);

    assert_eq!(
        result.expect_err("expected min price error").to_string(),
        cosmwasm_std::StdError::generic_err(format!(
            "Minimum price not met: {} required, {} provided",
            ctx.data.min_price.expect("expect min price is set"),
            price,
        ))
        .to_string()
    );
}

#[test]
fn on_list_plugin_returns_error_when_min_price_denom_mismatches() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "MinimumPrice",
            &Plugin::MinimumPrice {
                amount: Coin::new(150u128, "uxion"),
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&deps.api.addr_make("seller"), &[]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);
    let token_id = "token-1".to_string();
    let price = Coin::new(100u128, "uusdc");

    let result = contract.on_list_plugin(&token_id, &price, &None, &mut ctx);

    assert_eq!(
        result.expect_err("expected denom mismatch").to_string(),
        cosmwasm_std::StdError::generic_err(
            "ask price denom uusdc does not match minimum price denom uxion"
        )
        .to_string()
    );
}

#[test]
fn on_buy_plugin_runs_allowed_marketplace_and_royalty_plugins() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let buyer = deps.api.addr_make("buyer");
    let royalty_recipient = deps.api.addr_make("artist");
    let seller = deps.api.addr_make("seller");

    let price = Coin::new(100u128, "uxion");
    let nft_info = NftInfo {
        owner: seller.clone(),
        approvals: vec![],
        token_uri: None,
        extension: Empty::default(),
    };
    contract
        .config
        .listings
        .save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: seller.clone(),
                price: price.clone(),
                reserved: None,
            },
        )
        .unwrap();
    contract
        .config
        .cw721_config
        .nft_info
        .save(deps.as_mut().storage, "token-1", &nft_info)
        .unwrap();

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "AllowedMarketplaces",
            &Plugin::AllowedMarketplaces {
                marketplaces: vec![buyer.clone()],
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
                denoms: vec![Coin::new(0u128, "uxion")],
            },
        )
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "Royalty",
            &Plugin::Royalty {
                bps: 500,
                recipient: royalty_recipient.clone(),
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&buyer, &[Coin::new(100u128, "uxion")]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);

    let result = contract.on_buy_plugin("token-1", &None, &mut ctx).unwrap();

    assert!(result);
    assert_eq!(ctx.data.buyer, Some(Addr::unchecked(buyer.to_string())));
    assert_eq!(ctx.response.messages.len(), 1);
    match &ctx.response.messages[0].msg {
        cosmwasm_std::CosmosMsg::Bank(cosmwasm_std::BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, &royalty_recipient.to_string());
            assert_eq!(amount, &vec![Coin::new(5u128, "uxion")]);
        }
        msg => panic!("unexpected message: {:?}", msg),
    }
    let amount_attr = ctx
        .response
        .attributes
        .iter()
        .find(|attr| attr.key == "royalty_amount")
        .expect("royalty amount attr");
    assert_eq!(amount_attr.value, Coin::new(5u128, "uxion").to_string());
}

#[test]
fn on_buy_plugin_errors_when_currency_not_allowed() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let buyer = deps.api.addr_make("buyer");
    let seller = deps.api.addr_make("seller");

    let price = Coin::new(100u128, "uxion");
    let nft_info = NftInfo {
        owner: Addr::unchecked("nft-owner"),
        approvals: vec![],
        token_uri: None,
        extension: Empty::default(),
    };
    contract
        .config
        .listings
        .save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: seller.clone(),
                price: price.clone(),
                reserved: None,
            },
        )
        .unwrap();
    contract
        .config
        .cw721_config
        .nft_info
        .save(deps.as_mut().storage, "token-1", &nft_info)
        .unwrap();

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "AllowedCurrencies",
            &Plugin::AllowedCurrencies {
                denoms: vec![Coin::new(0u128, "uusdc")],
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&buyer, &[Coin::new(100u128, "uxion")]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);

    let result = contract.on_buy_plugin("token-1", &None, &mut ctx);

    assert_eq!(
        result
            .expect_err("expected currency not allowed")
            .to_string(),
        cosmwasm_std::StdError::generic_err("ask price currency is not allowed",).to_string()
    );
}

#[test]
fn on_buy_plugin_errors_when_marketplace_not_allowed() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let buyer = deps.api.addr_make("buyer");
    let seller = deps.api.addr_make("seller");

    let price = Coin::new(100u128, "uxion");
    let nft_info = NftInfo {
        owner: Addr::unchecked("nft-owner"),
        approvals: vec![],
        token_uri: None,
        extension: Empty::default(),
    };
    contract
        .config
        .listings
        .save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: seller.clone(),
                price: price.clone(),
                reserved: None,
            },
        )
        .unwrap();
    contract
        .config
        .cw721_config
        .nft_info
        .save(deps.as_mut().storage, "token-1", &nft_info)
        .unwrap();

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "AllowedMarketplaces",
            &Plugin::AllowedMarketplaces {
                marketplaces: vec![Addr::unchecked("someone-else")],
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&buyer, &[Coin::new(100u128, "uxion")]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);

    let result = contract.on_buy_plugin("token-1", &None, &mut ctx);

    assert_eq!(
        result
            .expect_err("expected marketplace not allowed")
            .to_string(),
        cosmwasm_std::StdError::generic_err("buyer is not an allowed marketplace",).to_string()
    );
}

#[test]
fn on_transfer_plugin_blocks_listed_tokens() {
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

    let env = env_at(1_000);
    let info = message_info(&seller, &[]);
    // let ctx = build_ctx(deps.as_ref(), env.clone(), info.clone());
    let mut msg = cw721::msg::Cw721ExecuteMsg::TransferNft {
        recipient: deps.api.addr_make("buyer").to_string(),
        token_id: "token-1".to_string(),
    };

    let mut err = contract
        .execute_pluggable(deps.as_mut(), &env, &info, msg)
        .expect_err("expected transfer block");

    assert_eq!(
        err.to_string(),
        cosmwasm_std::StdError::generic_err("cannot transfer a token while it is listed")
            .to_string()
    );
    msg = cw721::msg::Cw721ExecuteMsg::SendNft {
        contract: deps.api.addr_make("marketplace").to_string(),
        msg: Binary::default(),
        token_id: "token-1".to_string(),
    };

    err = contract
        .execute_pluggable(deps.as_mut(), &env, &info, msg)
        .expect_err("expected transfer block");

    assert_eq!(
        err.to_string(),
        cosmwasm_std::StdError::generic_err("cannot transfer a token while it is listed")
            .to_string()
    );
}

#[test]
fn on_transfer_plugin_allows_when_not_listed() {
    let deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let env = env_at(1_000);
    let info = message_info(&deps.api.addr_make("operator"), &[]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);

    assert!(
        contract
            .on_transfer_plugin("buyer", "token-1", &mut ctx)
            .is_ok()
    );
}

#[test]
fn on_reserve_plugin_respects_allowed_marketplaces_and_time_lock() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let reserver = deps.api.addr_make("reserver");

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "AllowedMarketplaces",
            &Plugin::AllowedMarketplaces {
                marketplaces: vec![reserver.clone()],
            },
        )
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "TimeLock",
            &Plugin::TimeLock {
                time: Duration::from_secs(2_000),
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&reserver, &[]);
    let mut ctx = build_ctx(deps.as_ref(), env.clone(), info);
    let reservation = ReserveMsg {
        reserver: Some(reserver.clone().to_string()),
        reserved_until: env.block.time.plus_seconds(1_500),
    };

    let result = contract
        .on_reserve_plugin("token-1", &reservation, &mut ctx)
        .unwrap();

    assert!(result);
    assert_eq!(
        ctx.data.reservation.unwrap().reserver.unwrap(),
        reserver.to_string()
    );
    assert_eq!(ctx.data.time_lock, Some(Duration::from_secs(2_000)));
}

#[test]
fn on_reserve_plugin_errors_for_disallowed_marketplace() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let reserver = deps.api.addr_make("reserver");

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "AllowedMarketplaces",
            &Plugin::AllowedMarketplaces {
                marketplaces: vec![Addr::unchecked("someone-else")],
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&reserver, &[]);
    let mut ctx = build_ctx(deps.as_ref(), env.clone(), info);
    let reservation = ReserveMsg {
        reserver: Some(reserver.clone().to_string()),
        reserved_until: env.block.time.plus_seconds(1_200),
    };

    let result = contract.on_reserve_plugin("token-1", &reservation, &mut ctx);

    assert_eq!(
        result
            .expect_err("expected marketplace not allowed")
            .to_string(),
        cosmwasm_std::StdError::generic_err("buyer is not an allowed marketplace",).to_string()
    );
}

#[test]
fn on_reserve_plugin_errors_when_time_lock_exceeded() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let reserver = deps.api.addr_make("reserver");

    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "AllowedMarketplaces",
            &Plugin::AllowedMarketplaces {
                marketplaces: vec![reserver.clone()],
            },
        )
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "TimeLock",
            &Plugin::TimeLock {
                time: Duration::from_secs(1_500),
            },
        )
        .unwrap();

    let env = env_at(1_000);
    let info = message_info(&reserver, &[]);
    let mut ctx = build_ctx(deps.as_ref(), env.clone(), info);
    let reservation = ReserveMsg {
        reserver: Some(reserver.clone().to_string()),
        reserved_until: env.block.time.plus_seconds(3_000),
    };

    let result = contract.on_reserve_plugin("token-1", &reservation, &mut ctx);

    assert_eq!(
        result.expect_err("expected time lock exceeded").to_string(),
        cosmwasm_std::StdError::generic_err(format!(
            "Reservation end time {} exceeds the collection time lock {}",
            reservation.reserved_until,
            Expiration::AtTime(
                ctx.env
                    .block
                    .time
                    .plus_seconds(ctx.data.time_lock.expect("time lock set").as_secs())
            )
        ))
        .to_string()
    );
}

#[test]
fn save_plugin_saves_all_plugins() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let owner = deps.api.addr_make("admin");
    {
        let deps_mut = deps.as_mut();
        CREATOR
            .initialize_owner(deps_mut.storage, deps_mut.api, Some(owner.as_str()))
            .unwrap();
    }

    let plugins = vec![
        Plugin::MinimumPrice {
            amount: Coin::new(50u128, "uxion"),
        },
        Plugin::NotBefore {
            time: Expiration::AtTime(Timestamp::from_seconds(500)),
        },
        Plugin::NotAfter {
            time: Expiration::AtTime(Timestamp::from_seconds(1_500)),
        },
        Plugin::AllowedCurrencies {
            denoms: vec![Coin::new(0u128, "uxion"), Coin::new(0u128, "uusdc")],
        },
    ];

    let env = env_at(1_000);
    let info = message_info(&owner, &[]);
    contract
        .save_plugin(deps.as_mut(), &env, &info, &plugins)
        .unwrap();

    let stored_plugins: Vec<Plugin> = contract
        .config
        .collection_plugins
        .range(
            deps.as_ref().storage,
            None,
            None,
            cosmwasm_std::Order::Ascending,
        )
        .map(|item| item.unwrap().1)
        .collect();
    assert_eq!(stored_plugins.len(), plugins.len());
    for plugin in plugins {
        assert!(stored_plugins.contains(&plugin));
    }
}

#[test]
fn remove_plugin_removes_specified_plugin() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let owner = deps.api.addr_make("admin");
    {
        let deps_mut = deps.as_mut();
        CREATOR
            .initialize_owner(deps_mut.storage, deps_mut.api, Some(owner.as_str()))
            .unwrap();
    }

    let plugins = vec![
        Plugin::MinimumPrice {
            amount: Coin::new(50u128, "uxion"),
        },
        Plugin::NotBefore {
            time: Expiration::AtTime(Timestamp::from_seconds(500)),
        },
        Plugin::NotAfter {
            time: Expiration::AtTime(Timestamp::from_seconds(1_500)),
        },
        Plugin::AllowedCurrencies {
            denoms: vec![Coin::new(0u128, "uxion"), Coin::new(0u128, "uusdc")],
        },
    ];

    let env = env_at(1_000);
    let info = message_info(&owner, &[]);
    contract
        .save_plugin(deps.as_mut(), &env, &info, &plugins)
        .unwrap();

    contract
        .remove_plugin(deps.as_mut(), &env, &info, &["NotAfter".to_string()])
        .unwrap();

    let stored_plugins: Vec<Plugin> = contract
        .config
        .collection_plugins
        .range(
            deps.as_ref().storage,
            None,
            None,
            cosmwasm_std::Order::Ascending,
        )
        .map(|item| item.unwrap().1)
        .collect();
    assert_eq!(stored_plugins.len(), plugins.len() - 1);
    assert!(!stored_plugins.contains(&Plugin::NotAfter {
        time: Expiration::AtTime(Timestamp::from_seconds(1_500))
    }));
}

#[test]
fn save_plugin_rejects_non_owner() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let owner = deps.api.addr_make("admin");
    {
        let deps_mut = deps.as_mut();
        CREATOR
            .initialize_owner(deps_mut.storage, deps_mut.api, Some(owner.as_str()))
            .unwrap();
    }

    let env = env_at(1_000);
    let non_owner = deps.api.addr_make("intruder");
    let info = message_info(&non_owner, &[]);
    let plugins = vec![Plugin::ExactPrice {
        amount: Coin::new(100u128, "uxion"),
    }];

    let err = contract
        .save_plugin(deps.as_mut(), &env, &info, &plugins)
        .expect_err("expected unauthorized");
    assert_eq!(
        err.to_string(),
        cosmwasm_std::StdError::generic_err("Caller is not the contract's current owner")
            .to_string()
    );
}

#[test]
fn transfer_and_send_disabled_while_listed() {
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

    let env = env_at(1_000);
    let info = message_info(&seller, &[]);
    // let ctx = build_ctx(deps.as_ref(), env.clone(), info.clone());
    let mut msg = cw721::msg::Cw721ExecuteMsg::TransferNft {
        recipient: deps.api.addr_make("buyer").to_string(),
        token_id: "token-1".to_string(),
    };

    let mut err = contract
        .execute_pluggable(deps.as_mut(), &env, &info, msg)
        .expect_err("expected transfer block");

    assert_eq!(
        err.to_string(),
        cosmwasm_std::StdError::generic_err("cannot transfer a token while it is listed")
            .to_string()
    );
    msg = cw721::msg::Cw721ExecuteMsg::SendNft {
        contract: deps.api.addr_make("marketplace").to_string(),
        msg: Binary::default(),
        token_id: "token-1".to_string(),
    };

    err = contract
        .execute_pluggable(deps.as_mut(), &env, &info, msg)
        .expect_err("expected transfer block");

    assert_eq!(
        err.to_string(),
        cosmwasm_std::StdError::generic_err("cannot transfer a token while it is listed")
            .to_string()
    );
}

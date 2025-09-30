#[cfg(test)]
mod plugins_test {
    use std::time::Duration;

    use crate::{
        plugin::{DefaultXionAssetContext, PluginCtx, default_plugins},
        state::Reserve,
    };
    use cosmwasm_std::{
        Addr, BankMsg, Coin, CosmosMsg, Deps, Empty, Env, MessageInfo, Response, Timestamp,
        testing::{message_info, mock_dependencies, mock_env},
    };
    use cw721::Expiration;

    fn env_at(time: u64) -> Env {
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(time);
        env
    }

    fn build_ctx<'a>(
        deps: Deps<'a>,
        env: Env,
        info: MessageInfo,
    ) -> PluginCtx<'a, DefaultXionAssetContext, Empty> {
        PluginCtx {
            deps,
            env,
            info,
            response: Response::default(),
            royalty: Default::default(),
            data: DefaultXionAssetContext::default(),
        }
    }

    #[test]
    fn exact_price_plugin_accepts_matching_funds() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("buyer"), &[Coin::new(100u128, "uatom")]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.ask_price = Some(Coin::new(100u128, "uatom"));

        assert!(default_plugins::exact_price_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn exact_price_plugin_errors_on_mismatch() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("buyer"), &[Coin::new(90u128, "uatom")]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.ask_price = Some(Coin::new(100u128, "uatom"));

        assert!(default_plugins::exact_price_plugin(&mut ctx).is_err());
    }

    #[test]
    fn minimum_price_plugin_allows_sufficient_price() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("seller"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.min_price = Some(Coin::new(80u128, "uatom"));
        ctx.data.ask_price = Some(Coin::new(100u128, "uatom"));

        assert!(default_plugins::min_price_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn minimum_price_plugin_errors_when_price_too_low() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("seller"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.min_price = Some(Coin::new(120u128, "uatom"));
        ctx.data.ask_price = Some(Coin::new(100u128, "uatom"));

        assert!(default_plugins::min_price_plugin(&mut ctx).is_err());
    }

    #[test]
    fn not_before_plugin_errors_when_time_not_reached() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("seller"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.not_before = Expiration::AtTime(Timestamp::from_seconds(1_500));

        assert!(default_plugins::not_before_plugin(&mut ctx).is_err());
    }

    #[test]
    fn not_before_plugin_allows_after_time() {
        let deps = mock_dependencies();
        let env = env_at(2_000);
        let info = message_info(&deps.api.addr_make("seller"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.not_before = Expiration::AtTime(Timestamp::from_seconds(1_500));

        assert!(default_plugins::not_before_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn not_after_plugin_allows_before_deadline() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("seller"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.not_after = Expiration::AtTime(Timestamp::from_seconds(1_500));

        assert!(default_plugins::not_after_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn not_after_plugin_errors_when_expired() {
        let deps = mock_dependencies();
        let env = env_at(2_000);
        let info = message_info(&deps.api.addr_make("seller"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.not_after = Expiration::AtTime(Timestamp::from_seconds(1_500));

        assert!(default_plugins::not_after_plugin(&mut ctx).is_err());
    }

    #[test]
    fn time_lock_plugin_allows_reservation_within_limit() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("marketplace"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.time_lock = Some(Duration::from_secs(2_000));
        ctx.data.reservation = Some(Reserve {
            reserver: Addr::unchecked("reserver"),
            reserved_until: Expiration::AtTime(Timestamp::from_seconds(1_500)),
        });

        assert!(default_plugins::time_lock_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn time_lock_plugin_errors_when_reservation_exceeds_limit() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("marketplace"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.time_lock = Some(Duration::from_secs(2_000));
        ctx.data.reservation = Some(Reserve {
            reserver: Addr::unchecked("reserver"),
            reserved_until: Expiration::AtTime(Timestamp::from_seconds(3_500)),
        });

        assert!(default_plugins::time_lock_plugin(&mut ctx).is_err());
    }

    #[test]
    fn royalty_plugin_creates_deduction_and_message() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(
            &deps.api.addr_make("buyer"),
            &[Coin::new(1_000u128, "uxion")],
        );
        let mut ctx = build_ctx(deps.as_ref(), env, info);

        ctx.data.ask_price = Some(Coin::new(1_000u128, "uxion"));
        ctx.royalty.collection_royalty_recipient = Some(Addr::unchecked("artist"));
        ctx.royalty.collection_royalty_bps = Some(500);
        ctx.royalty.primary_complete = true;

        assert!(default_plugins::royalty_plugin(&mut ctx).is_ok());
        let attr = &ctx.response.attributes[0];
        assert_eq!(attr.key, "royalty_amount");
        assert_eq!(attr.value, Coin::new(50u128, "uxion").to_string());
        let attr = &ctx.response.attributes[1];
        assert_eq!(attr.key, "royalty_recipient");
        assert_eq!(attr.value, Addr::unchecked("artist").to_string());

        assert_eq!(ctx.response.messages.len(), 1);
        match &ctx.response.messages[0].msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address, "artist");
                assert_eq!(amount, &vec![Coin::new(50u128, "uxion")]);
            }
            other => panic!("unexpected message: {:?}", other),
        }
    }

    #[test]
    fn royalty_plugin_errors_when_missing_funds() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("buyer"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.ask_price = Some(Coin::new(1_000u128, "uxion"));
        ctx.royalty.collection_royalty_recipient = Some(Addr::unchecked("artist"));
        ctx.royalty.collection_royalty_bps = Some(500);
        ctx.royalty.primary_complete = true;

        assert!(default_plugins::royalty_plugin(&mut ctx).is_err());
    }

    #[test]
    fn allowed_marketplaces_plugin_accepts_allowed_buyer() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("marketplace"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.allowed_marketplaces = Some(vec![deps.api.addr_make("marketplace")]);

        assert!(default_plugins::allowed_marketplaces_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn allowed_marketplaces_plugin_rejects_disallowed_buyer() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("unauthorized"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.allowed_marketplaces = Some(vec![Addr::unchecked("allowed")]);
        ctx.data.buyer = Some(Addr::unchecked("unauthorized"));

        assert!(default_plugins::allowed_marketplaces_plugin(&mut ctx).is_err());
    }

    #[test]
    fn allowed_currencies_plugin_accepts_configured_currencies() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("buyer"), &[Coin::new(100u128, "uxion")]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.allowed_currencies =
            Some(vec![Coin::new(0u128, "uxion"), Coin::new(0u128, "uusdc")]);
        ctx.data.ask_price = Some(Coin::new(100u128, "uxion"));
        ctx.data.min_price = Some(Coin::new(50u128, "uxion"));

        assert!(default_plugins::allowed_currencies_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn allowed_currencies_plugin_rejects_unlisted_currency() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("buyer"), &[Coin::new(100u128, "uxion")]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.data.allowed_currencies = Some(vec![Coin::new(0u128, "uusdc")]);
        ctx.data.ask_price = Some(Coin::new(100u128, "uxion"));

        assert!(default_plugins::allowed_currencies_plugin(&mut ctx).is_err());
    }

    #[test]
    fn transfer_enabled_plugin_allows_when_no_royalty() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("anyone"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);

        assert!(default_plugins::is_transfer_enabled_plugin(&mut ctx).is_ok());
    }

    #[test]
    fn transfer_enabled_plugin_errors_when_royalty_set() {
        let deps = mock_dependencies();
        let env = env_at(1_000);
        let info = message_info(&deps.api.addr_make("anyone"), &[]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        ctx.royalty.collection_royalty_bps = Some(500);
        ctx.royalty.collection_royalty_recipient = Some(Addr::unchecked("artist"));

        assert!(default_plugins::is_transfer_enabled_plugin(&mut ctx).is_err());
    }
}

#[cfg(test)]
mod asset_pluggable_tests {
    use std::time::Duration;

    use crate::{
        msg::AssetExtensionExecuteMsg,
        plugin::{DefaultXionAssetContext, PluggableAsset, Plugin, PluginCtx, RoyaltyInfo},
        state::{ListingInfo, Reserve},
        traits::{AssetContract, DefaultAssetContract},
    };
    use cosmwasm_std::{
        Addr, Coin, Deps, Empty, Env, MessageInfo, Response, Timestamp,
        testing::{message_info, mock_dependencies, mock_env},
    };
    use cw721::{Expiration, state::NftInfo};

    fn env_at(time: u64) -> Env {
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(time);
        env
    }

    fn build_ctx<'a>(
        deps: Deps<'a>,
        env: Env,
        info: MessageInfo,
    ) -> PluginCtx<'a, DefaultXionAssetContext, Empty> {
        PluginCtx {
            deps,
            env,
            info,
            response: Response::default(),
            royalty: RoyaltyInfo::default(),
            data: DefaultXionAssetContext::default(),
        }
    }

    #[test]
    fn on_list_plugin_runs_all_configured_plugins() {
        let mut deps = mock_dependencies();
        let contract =
            AssetContract::<'_, Empty, Empty, Empty, Empty, AssetExtensionExecuteMsg>::default();

        let min_price = Coin::new(50 as u128, "uxion");
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
        let allowed = ctx.data.allowed_currencies.expect("allowed currencies");
        assert!(allowed.iter().any(|coin| coin.denom == "uxion"));
    }

    #[test]
    fn on_list_plugin_returns_error_when_not_after_fails() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
    fn on_buy_plugin_runs_allowed_marketplace_and_royalty_plugins() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

        let buyer = deps.api.addr_make("buyer");
        let royalty_recipient = deps.api.addr_make("artist");
        let seller = deps.api.addr_make("seller");

        let price = Coin::new(100u128, "uxion");
        contract
            .config
            .listings
            .save(
                deps.as_mut().storage,
                "token-1",
                &ListingInfo {
                    id: "token-1".to_string(),
                    price: price.clone(),
                    seller: seller.clone(),
                    reserved: None,
                    nft_info: NftInfo {
                        owner: seller,
                        approvals: vec![],
                        token_uri: None,
                        extension: Empty::default(),
                    },
                },
            )
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
                    on_primary: true,
                },
            )
            .unwrap();

        let env = env_at(1_000);
        let info = message_info(&buyer, &[Coin::new(100u128, "uxion")]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);

        let result = contract
            .on_buy_plugin(&"token-1".to_string(), &None, &mut ctx)
            .unwrap();

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
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

        let buyer = deps.api.addr_make("buyer");
        let seller = deps.api.addr_make("seller");

        let price = Coin::new(100u128, "uxion");
        contract
            .config
            .listings
            .save(
                deps.as_mut().storage,
                "token-1",
                &ListingInfo {
                    id: "token-1".to_string(),
                    price: price.clone(),
                    seller,
                    reserved: None,
                    nft_info: NftInfo {
                        owner: Addr::unchecked("nft-owner"),
                        approvals: vec![],
                        token_uri: None,
                        extension: Empty::default(),
                    },
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
                    denoms: vec![Coin::new(0u128, "uusdc")],
                },
            )
            .unwrap();

        let env = env_at(1_000);
        let info = message_info(&buyer, &[Coin::new(100u128, "uxion")]);
        let mut ctx = build_ctx(deps.as_ref(), env, info);

        let result = contract.on_buy_plugin(&"token-1".to_string(), &None, &mut ctx);

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
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

        let buyer = deps.api.addr_make("buyer");
        let seller = deps.api.addr_make("seller");

        let price = Coin::new(100u128, "uxion");
        contract
            .config
            .listings
            .save(
                deps.as_mut().storage,
                "token-1",
                &ListingInfo {
                    id: "token-1".to_string(),
                    price: price.clone(),
                    seller,
                    reserved: None,
                    nft_info: NftInfo {
                        owner: Addr::unchecked("nft-owner"),
                        approvals: vec![],
                        token_uri: None,
                        extension: Empty::default(),
                    },
                },
            )
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

        let result = contract.on_buy_plugin(&"token-1".to_string(), &None, &mut ctx);

        assert_eq!(
            result
                .expect_err("expected marketplace not allowed")
                .to_string(),
            cosmwasm_std::StdError::generic_err("buyer is not an allowed marketplace",).to_string()
        );
    }

    #[test]
    fn on_reserve_plugin_respects_allowed_marketplaces_and_time_lock() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        let reservation = Reserve {
            reserver: reserver.clone(),
            reserved_until: Expiration::AtTime(Timestamp::from_seconds(1_500)),
        };

        let result = contract
            .on_reserve_plugin(&"token-1".to_string(), &reservation, &mut ctx)
            .unwrap();

        assert!(result);
        assert_eq!(ctx.data.reservation.as_ref().unwrap().reserver, reserver);
        assert_eq!(
            ctx.data.time_lock,
            Some(Duration::from_secs(2_000))
        );
    }

    #[test]
    fn on_reserve_plugin_errors_for_disallowed_marketplace() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        let reservation = Reserve {
            reserver: reserver.clone(),
            reserved_until: Expiration::AtTime(Timestamp::from_seconds(1_200)),
        };

        let result = contract.on_reserve_plugin(&"token-1".to_string(), &reservation, &mut ctx);

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
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
        let mut ctx = build_ctx(deps.as_ref(), env, info);
        let reservation = Reserve {
            reserver: reserver.clone(),
            reserved_until: Expiration::AtTime(Timestamp::from_seconds(3_000)),
        };

        let result = contract.on_reserve_plugin(&"token-1".to_string(), &reservation, &mut ctx);

        assert_eq!(
            result
                .expect_err("expected time lock exceeded")
                .to_string(),
            cosmwasm_std::StdError::generic_err(format!(
                "Reservation end time {} exceeds the collection time lock {}",
                reservation.reserved_until, Expiration::AtTime(ctx.env.block.time.plus_seconds(ctx.data.time_lock.expect("time lock set").as_secs()))
            ))
            .to_string()
        );
    }

    #[test]
    fn save_plugin_saves_all_plugins() {
        let mut deps = mock_dependencies();
        let mut contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
        let info = message_info(&deps.api.addr_make("admin"), &[]);
        contract
            .save_plugin(deps.as_mut(), &env, &info, &plugins)
            .unwrap();

        let stored_plugins: Vec<Plugin> = contract
            .config
            .collection_plugins
            .range(deps.as_ref().storage, None, None, cosmwasm_std::Order::Ascending)
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
        let mut contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
        let info = message_info(&deps.api.addr_make("admin"), &[]);
        contract
            .save_plugin(deps.as_mut(), &env, &info, &plugins)
            .unwrap();

        contract
            .remove_plugin(deps.as_mut(), &env, &info, &["NotAfter".to_string()].into())
            .unwrap();

        let stored_plugins: Vec<Plugin> = contract
            .config
            .collection_plugins
            .range(deps.as_ref().storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|item| item.unwrap().1)
            .collect();
        assert_eq!(stored_plugins.len(), plugins.len() - 1);
        assert!(!stored_plugins.contains(&Plugin::NotAfter {
            time: Expiration::AtTime(Timestamp::from_seconds(1_500))
        }));
    }
}

#[cfg(test)]
mod query_test {
    use crate::{
        msg::AssetExtensionQueryMsg,
        plugin::Plugin,
        state::ListingInfo,
        traits::DefaultAssetContract,
    };
    use cosmwasm_std::{
        from_json,
        testing::{mock_dependencies, mock_env},
        Coin, Empty,
    };
    use cw721::{state::NftInfo, traits::Cw721Query};

    #[test]
    fn get_listing_returns_saved_listing() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

        let seller = deps.api.addr_make("seller");
        let listing = ListingInfo {
            id: "token-1".to_string(),
            price: Coin::new(100u128, "uxion"),
            seller: seller.clone(),
            reserved: None,
            nft_info: NftInfo {
                owner: seller,
                approvals: vec![],
                token_uri: None,
                extension: Empty::default(),
            },
        };

        contract
            .config
            .listings
            .save(deps.as_mut().storage, "token-1", &listing)
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

        let fetched: ListingInfo<Empty> = from_json(binary).unwrap();
        assert_eq!(fetched, listing);
    }

    #[test]
    fn get_listings_by_seller_supports_pagination() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

        let seller = deps.api.addr_make("seller");
        let other = deps.api.addr_make("other");

        for (id, owner) in [
            ("token-1", seller.clone()),
            ("token-2", seller.clone()),
            ("token-3", seller.clone()),
            ("token-4", other.clone()),
        ] {
            let listing = ListingInfo {
                id: id.to_string(),
                price: Coin::new(50u128, "uxion"),
                seller: owner.clone(),
                reserved: None,
                nft_info: NftInfo {
                    owner,
                    approvals: vec![],
                    token_uri: None,
                    extension: Empty::default(),
                },
            };
            contract
                .config
                .listings
                .save(deps.as_mut().storage, id, &listing)
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
        let first_page: Vec<ListingInfo<Empty>> = from_json(first_page).unwrap();
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
        let second_page: Vec<ListingInfo<Empty>> = from_json(second_page).unwrap();
        assert_eq!(second_page.len(), 2);
        assert_eq!(second_page[0].id, "token-2");
        assert_eq!(second_page[1].id, "token-3");
        assert!(second_page.iter().all(|listing| listing.seller == seller));
    }

    #[test]
    fn get_all_listings_supports_pagination() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

        let seller = deps.api.addr_make("seller");
        let other = deps.api.addr_make("other");

        for (id, owner) in [
            ("token-1", seller.clone()),
            ("token-2", seller.clone()),
            ("token-3", seller.clone()),
            ("token-4", other.clone()),
        ] {
            let listing = ListingInfo {
                id: id.to_string(),
                price: Coin::new(50u128, "uxion"),
                seller: owner.clone(),
                reserved: None,
                nft_info: NftInfo {
                    owner,
                    approvals: vec![],
                    token_uri: None,
                    extension: Empty::default(),
                },
            };
            contract
                .config
                .listings
                .save(deps.as_mut().storage, id, &listing)
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
        let first_page: Vec<ListingInfo<Empty>> = from_json(first_page).unwrap();
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
        let second_page: Vec<ListingInfo<Empty>> = from_json(second_page).unwrap();
        assert_eq!(second_page.len(), 3);
        assert_eq!(second_page[0].id, "token-2");
        assert_eq!(second_page[1].id, "token-3");
        assert_eq!(second_page[2].id, "token-4");
    }
    
    #[test]
    fn get_collection_plugins_returns_all_plugins() {
        let mut deps = mock_dependencies();
        let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> =
            Default::default();

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
}

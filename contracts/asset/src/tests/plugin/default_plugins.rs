use std::time::Duration;

use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, Deps, Empty, Env, MessageInfo, Response, Timestamp,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::Expiration;

use crate::{
    default_plugins,
    msg::ReserveMsg,
    plugin::{DefaultXionAssetContext, PluginCtx},
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
        royalty: Default::default(),
        data: DefaultXionAssetContext::default(),
        deductions: vec![],
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
    let mut ctx = build_ctx(deps.as_ref(), env.clone(), info);
    ctx.data.time_lock = Some(Duration::from_secs(2_000));
    ctx.data.reservation = Some(ReserveMsg {
        reserver: Some(Addr::unchecked("reserver").to_string()),
        reserved_until: env.block.time.plus_seconds(600),
    });

    assert!(default_plugins::time_lock_plugin(&mut ctx).is_ok());
}

#[test]
fn time_lock_plugin_errors_when_reservation_exceeds_limit() {
    let deps = mock_dependencies();
    let env = env_at(1_000);
    let info = message_info(&deps.api.addr_make("marketplace"), &[]);
    let mut ctx = build_ctx(deps.as_ref(), env.clone(), info);
    ctx.data.time_lock = Some(Duration::from_secs(
        env.block.time.plus_seconds(2_000).seconds(),
    ));
    ctx.data.reservation = Some(ReserveMsg {
        reserver: Some(Addr::unchecked("reserver").to_string()),
        reserved_until: env.block.time.plus_seconds(6000),
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

#[allow(dead_code)]
fn royalty_plugin_rounds_up_small_amounts() {
    let deps = mock_dependencies();
    let env = env_at(1_000);
    let info = message_info(&deps.api.addr_make("buyer"), &[Coin::new(1u128, "uxion")]);
    let mut ctx = build_ctx(deps.as_ref(), env, info);

    ctx.data.ask_price = Some(Coin::new(1u128, "uxion"));
    ctx.royalty.collection_royalty_recipient = Some(Addr::unchecked("artist"));
    ctx.royalty.collection_royalty_bps = Some(1); // 0.01%
    ctx.royalty.primary_complete = true;

    assert!(default_plugins::royalty_plugin(&mut ctx).is_ok());
    let attr = &ctx.response.attributes[0];
    assert_eq!(attr.key, "royalty_amount");
    assert_eq!(attr.value, Coin::new(1u128, "uxion").to_string());

    match &ctx.response.messages[0].msg {
        CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
            assert_eq!(to_address, "artist");
            assert_eq!(amount, &vec![Coin::new(1u128, "uxion")]);
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
    ctx.data.allowed_currencies = Some(vec![Coin::new(0u128, "uxion"), Coin::new(0u128, "uusdc")]);
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

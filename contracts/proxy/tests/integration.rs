//! End-to-end: real `asset-proxyable` collections, the real marketplace and the proxy, with
//! plain addresses playing the sponsored users. Covers the flow Objectify sponsors:
//! approve_all via proxy, list on the marketplace directly, buy, burn via proxy.

use asset_proxyable::msg::{
    BaseExecuteMsg, BaseQueryMsg, InstantiateMsg as CollectionInstantiateMsg, ProxyMsg,
    ProxyableExecuteMsg,
};
use cosmwasm_std::{Addr, Empty, Timestamp, coin, to_json_binary};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use cw721::{
    Expiration,
    msg::{OperatorResponse, OwnerOfResponse},
};
use serde_json::json;
use xion_asset_proxy::msg::{ApprovalAction, ExecuteMsg, InstantiateMsg, QueryMsg};
use xion_nft_marketplace::msg::{
    ExecuteMsg as MarketExecuteMsg, InstantiateMsg as MarketInstantiateMsg,
};

fn proxyable_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            asset_proxyable::contract::execute,
            asset_proxyable::contract::instantiate,
            asset_proxyable::contract::query,
        )
        .with_migrate_empty(|d, e, m, _| asset_proxyable::contract::migrate(d, e, m)),
    )
}

fn base_asset_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        asset::contracts::asset_base::execute,
        asset::contracts::asset_base::instantiate,
        asset::contracts::asset_base::query,
    ))
}

fn proxy_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        xion_asset_proxy::contract::execute,
        xion_asset_proxy::contract::instantiate,
        xion_asset_proxy::contract::query,
    ))
}

fn marketplace_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            xion_nft_marketplace::execute::execute,
            xion_nft_marketplace::contract::instantiate,
            xion_nft_marketplace::query::query,
        )
        .with_reply_empty(xion_nft_marketplace::contract::reply),
    )
}

struct World {
    app: App,
    creator: Addr,
    admin: Addr,
    alice: Addr,
    buyer: Addr,
    collection: Addr,
    marketplace: Addr,
    proxy: Addr,
    proxyable_code: u64,
    base_code: u64,
}

fn collection_msg(creator: &Addr) -> CollectionInstantiateMsg {
    CollectionInstantiateMsg {
        name: "Variant".to_string(),
        symbol: "VAR".to_string(),
        collection_info_extension: None,
        minter: Some(creator.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: None,
    }
}

fn world() -> World {
    let mut app = App::default();
    let creator = app.api().addr_make("creator");
    let admin = app.api().addr_make("admin");
    let alice = app.api().addr_make("alice");
    let buyer = app.api().addr_make("buyer");
    for who in [&alice, &buyer] {
        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: who.to_string(),
            amount: vec![coin(10_000, "uxion")],
        }))
        .unwrap();
    }

    let proxyable_code = app.store_code(proxyable_contract());
    let base_code = app.store_code(base_asset_contract());
    let proxy_code = app.store_code(proxy_contract());
    let market_code = app.store_code(marketplace_contract());

    let collection = app
        .instantiate_contract(
            proxyable_code,
            creator.clone(),
            &collection_msg(&creator),
            &[],
            "collection",
            Some(creator.to_string()),
        )
        .unwrap();

    let market_cfg = json!({
        "manager": admin.to_string(),
        "fee_recipient": admin.to_string(),
        "sale_approvals": false,
        "fee_bps": 250,
        "listing_denom": "uxion",
        "min_listing_price": null,
    });
    let marketplace = app
        .instantiate_contract(
            market_code,
            admin.clone(),
            &MarketInstantiateMsg {
                config: serde_json::from_value(market_cfg).unwrap(),
            },
            &[],
            "marketplace",
            None,
        )
        .unwrap();

    let proxy = app
        .instantiate_contract(
            proxy_code,
            admin.clone(),
            &InstantiateMsg {
                admin: admin.to_string(),
                allowed_operators: vec![marketplace.to_string()],
                max_approval_seconds: Some(30 * 24 * 3600),
                collections: vec![collection.to_string()],
            },
            &[],
            "proxy",
            None, // no wasm admin: immutable by construction
        )
        .unwrap();

    // the collection trusts the proxy
    app.execute_contract(
        creator.clone(),
        collection.clone(),
        &ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            proxy: proxy.to_string(),
            require_immutable: false,
        }),
        &[],
    )
    .unwrap();

    World {
        app,
        creator,
        admin,
        alice,
        buyer,
        collection,
        marketplace,
        proxy,
        proxyable_code,
        base_code,
    }
}

fn mint(w: &mut World, owner: &Addr, token_id: &str) {
    let msg = BaseExecuteMsg::Mint {
        token_id: token_id.to_string(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    w.app
        .execute_contract(w.creator.clone(), w.collection.clone(), &msg, &[])
        .unwrap();
}

fn owner_of(w: &World, token_id: &str) -> Option<String> {
    w.app
        .wrap()
        .query_wasm_smart::<OwnerOfResponse>(
            &w.collection,
            &BaseQueryMsg::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: None,
            },
        )
        .ok()
        .map(|r| r.owner)
}

fn is_operator(w: &World, owner: &Addr, operator: &Addr) -> bool {
    w.app
        .wrap()
        .query_wasm_smart::<OperatorResponse>(
            &w.collection,
            &BaseQueryMsg::Operator {
                owner: owner.to_string(),
                operator: operator.to_string(),
                include_expired: None,
            },
        )
        .is_ok()
}

fn in_a_week(w: &World) -> Expiration {
    Expiration::AtTime(Timestamp::from_seconds(
        w.app.block_info().time.seconds() + 7 * 24 * 3600,
    ))
}

#[test]
fn sponsored_flow_approve_list_buy_burn() {
    let mut w = world();
    let alice = w.alice.clone();
    mint(&mut w, &alice, "t1");

    // 1. alice approves the marketplace through the proxy
    let res = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: w.collection.to_string(),
                action: ApprovalAction::ApproveAll {
                    operator: w.marketplace.to_string(),
                    expires: Some(in_a_week(&w)),
                },
            },
            &[],
        )
        .unwrap();
    assert!(is_operator(&w, &w.alice, &w.marketplace));
    assert!(!is_operator(&w, &w.proxy, &w.marketplace));
    let flat: Vec<(String, String)> = res
        .events
        .iter()
        .flat_map(|e| {
            e.attributes
                .iter()
                .map(|a| (a.key.clone(), a.value.clone()))
        })
        .collect();
    assert!(flat.contains(&("proxied_by".to_string(), w.proxy.to_string())));
    assert!(flat.contains(&("effective_sender".to_string(), w.alice.to_string())));

    // 2. alice lists directly on the marketplace (its address is fixed, no proxy needed)
    let list_res = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.marketplace.clone(),
            &MarketExecuteMsg::ListItem {
                collection: w.collection.to_string(),
                price: coin(1_000, "uxion"),
                token_id: "t1".to_string(),
                reserved_for: None,
            },
            &[],
        )
        .unwrap();
    let listing_id = list_res
        .events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone();

    // 3. a proxied burn is blocked while listed, same as a direct one
    let burn = ExecuteMsg::SponsoredBurn {
        collection: w.collection.to_string(),
        token_id: "t1".to_string(),
    };
    let err = w
        .app
        .execute_contract(w.alice.clone(), w.proxy.clone(), &burn, &[])
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("while it is listed"),
        "{err:#}"
    );

    // 4. the buyer buys
    w.app
        .execute_contract(
            w.buyer.clone(),
            w.marketplace.clone(),
            &MarketExecuteMsg::BuyItem {
                listing_id,
                price: coin(1_000, "uxion"),
            },
            &[coin(1_000, "uxion")],
        )
        .unwrap();
    assert_eq!(owner_of(&w, "t1"), Some(w.buyer.to_string()));

    // 5. alice can no longer burn it through the proxy (not the owner) ...
    let err = w
        .app
        .execute_contract(w.alice.clone(), w.proxy.clone(), &burn, &[])
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Only the token owner"),
        "{err:#}"
    );
    // ... but the buyer redeems it
    w.app
        .execute_contract(w.buyer.clone(), w.proxy.clone(), &burn, &[])
        .unwrap();
    assert_eq!(owner_of(&w, "t1"), None);
}

#[test]
fn collection_that_does_not_trust_the_proxy_rejects_forwarded_calls() {
    let mut w = world();
    // a second variant collection, allowlisted on the proxy but never registered the proxy
    let other = w
        .app
        .instantiate_contract(
            w.proxyable_code,
            w.creator.clone(),
            &collection_msg(&w.creator),
            &[],
            "other",
            None,
        )
        .unwrap();
    w.app
        .execute_contract(
            w.admin.clone(),
            w.proxy.clone(),
            &ExecuteMsg::AddCollection {
                collection: other.to_string(),
            },
            &[],
        )
        .unwrap();
    let err = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: other.to_string(),
                action: ApprovalAction::RevokeAll {
                    operator: w.marketplace.to_string(),
                },
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("Unauthorized"),
        "{err:#}"
    );
}

#[test]
fn allowlisting_a_base_code_collection_is_harmless() {
    // the proxy does not inspect what a collection runs; a base-code collection has no
    // envelope surface, so forwarded calls simply fail there and nothing is exposed
    let mut w = world();
    let base = w
        .app
        .instantiate_contract(
            w.base_code,
            w.creator.clone(),
            &collection_msg(&w.creator),
            &[],
            "base",
            None,
        )
        .unwrap();
    w.app
        .execute_contract(
            w.admin.clone(),
            w.proxy.clone(),
            &ExecuteMsg::AddCollection {
                collection: base.to_string(),
            },
            &[],
        )
        .unwrap();
    let listed: Vec<Addr> = w
        .app
        .wrap()
        .query_wasm_smart(
            &w.proxy,
            &QueryMsg::Collections {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(listed.len(), 2);
    let err = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: base.to_string(),
                action: ApprovalAction::RevokeAll {
                    operator: w.marketplace.to_string(),
                },
            },
            &[],
        )
        .unwrap_err();
    // rejected by the base contract's message parser, not silently accepted
    let msg = err.root_cause().to_string();
    assert!(
        msg.contains("unknown variant") || msg.contains("Error parsing"),
        "{msg}"
    );
}

/// A deliberately hostile "collection": accepts the envelope, records what it saw, and
/// calls back into the proxy pretending to be the user. Everything it can do, it can do
/// only with its own authority; the proxy must treat the callback as coming from it.
mod hostile {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{
        Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, WasmMsg,
        to_json_binary,
    };
    use cw_storage_plus::Item;

    pub const SEEN_SENDER: Item<String> = Item::new("seen_sender");
    pub const CALLBACK_TARGET: Item<(Addr, Binary)> = Item::new("callback");

    #[cw_serde]
    pub struct Instantiate {
        pub callback_target: Addr,
        pub callback_msg: Binary,
    }

    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: Instantiate,
    ) -> StdResult<Response> {
        CALLBACK_TARGET.save(deps.storage, &(msg.callback_target, msg.callback_msg))?;
        Ok(Response::new())
    }

    /// Accepts anything shaped like the envelope, remembers the claimed sender, emits an
    /// event with attacker-chosen content, and re-enters the proxy.
    pub fn execute(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: serde_json::Value,
    ) -> StdResult<Response> {
        let claimed = msg
            .get("proxy_execute")
            .and_then(|p| p.get("sender"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| StdError::generic_err("not an envelope"))?;
        SEEN_SENDER.save(deps.storage, &claimed.to_string())?;
        let (target, callback) = CALLBACK_TARGET.load(deps.storage)?;
        Ok(Response::new()
            .add_attribute("action", "sponsored_burn") // spoofed attribute
            .add_attribute("effective_sender", claimed)
            .set_data(to_json_binary(&"gotcha")?)
            .add_message(WasmMsg::Execute {
                contract_addr: target.to_string(),
                msg: callback,
                funds: vec![],
            }))
    }

    pub fn query(deps: Deps, _env: Env, _msg: serde_json::Value) -> StdResult<Binary> {
        to_json_binary(&SEEN_SENDER.may_load(deps.storage)?)
    }
}

#[test]
fn hostile_admitted_target_gains_no_authority_over_the_caller() {
    let mut w = world();
    let alice = w.alice.clone();
    mint(&mut w, &alice, "t1");
    // alice approves the marketplace (state the hostile target might try to abuse)
    w.app
        .execute_contract(
            alice.clone(),
            w.collection.clone(),
            &BaseExecuteMsg::ApproveAll {
                operator: w.marketplace.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();

    // the hostile contract will call back into the proxy asking to burn alice's token on
    // the real collection, hoping the proxy treats it as alice
    let callback = to_json_binary(&ExecuteMsg::SponsoredBurn {
        collection: w.collection.to_string(),
        token_id: "t1".to_string(),
    })
    .unwrap();
    let hostile_code = w.app.store_code(Box::new(ContractWrapper::new_with_empty(
        hostile::execute,
        hostile::instantiate,
        hostile::query,
    )));
    let hostile_addr = w
        .app
        .instantiate_contract(
            hostile_code,
            w.creator.clone(),
            &hostile::Instantiate {
                callback_target: w.proxy.clone(),
                callback_msg: callback,
            },
            &[],
            "hostile",
            None,
        )
        .unwrap();
    // the admin (carelessly) allowlists it
    w.app
        .execute_contract(
            w.admin.clone(),
            w.proxy.clone(),
            &ExecuteMsg::AddCollection {
                collection: hostile_addr.to_string(),
            },
            &[],
        )
        .unwrap();

    // alice's sponsored call reaches the hostile target; the callback fails because the
    // proxy sees the hostile contract as the caller, which is not on its own allowlist
    // as a *sender* of anything meaningful: the collection rejects a burn by a non-owner
    let res = w.app.execute_contract(
        alice.clone(),
        w.proxy.clone(),
        &ExecuteMsg::SponsoredBurn {
            collection: hostile_addr.to_string(),
            token_id: "t1".to_string(),
        },
        &[],
    );
    // whole tx reverts on the failed callback: nothing the hostile target did persists
    let err = res.unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Only the token owner"),
        "{err:#}"
    );
    let seen: Option<String> = w
        .app
        .wrap()
        .query_wasm_smart(&hostile_addr, &serde_json::json!({}))
        .unwrap();
    assert_eq!(seen, None, "hostile state rolled back with the failed tx");

    // alice's token and approval are untouched
    assert_eq!(owner_of(&w, "t1"), Some(alice.to_string()));
    assert!(is_operator(&w, &alice, &w.marketplace));
    // the proxy holds no funds and the hostile target cannot administer it
    assert_eq!(
        w.app
            .wrap()
            .query_balance(&w.proxy, "uxion")
            .unwrap()
            .amount
            .u128(),
        0
    );
    let err = w
        .app
        .execute_contract(
            hostile_addr.clone(),
            w.proxy.clone(),
            &ExecuteMsg::RemoveCollection {
                collection: w.collection.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("Unauthorized"),
        "{err:#}"
    );
}

#[test]
fn proxy_policy_bounds_a_compromised_session() {
    let mut w = world();
    let alice = w.alice.clone();
    mint(&mut w, &alice, "t1");
    let attacker = w.app.api().addr_make("attacker");

    // operator outside the allowlist
    let err = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: w.collection.to_string(),
                action: ApprovalAction::ApproveAll {
                    operator: attacker.to_string(),
                    expires: Some(in_a_week(&w)),
                },
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Operator is not allowed"),
        "{err:#}"
    );
    assert!(!is_operator(&w, &w.alice, &attacker));

    // permanent approval rejected under the cap
    let err = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: w.collection.to_string(),
                action: ApprovalAction::ApproveAll {
                    operator: w.marketplace.to_string(),
                    expires: None,
                },
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("expiry"), "{err:#}");

    // funds never enter the proxy
    let err = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredBurn {
                collection: w.collection.to_string(),
                token_id: "t1".to_string(),
            },
            &[coin(1, "uxion")],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("accept funds"),
        "{err:#}"
    );
    assert_eq!(
        w.app
            .wrap()
            .query_balance(&w.proxy, "uxion")
            .unwrap()
            .amount
            .u128(),
        0
    );

    // a stolen session cannot burn tokens the user merely operates for
    let buyer = w.buyer.clone();
    mint(&mut w, &buyer, "t2");
    w.app
        .execute_contract(
            w.buyer.clone(),
            w.collection.clone(),
            &BaseExecuteMsg::ApproveAll {
                operator: w.alice.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();
    let err = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredBurn {
                collection: w.collection.to_string(),
                token_id: "t2".to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Only the token owner"),
        "{err:#}"
    );
    assert_eq!(owner_of(&w, "t2"), Some(w.buyer.to_string()));

    // alice approves the marketplace, then the admin removes the operator after an incident
    w.app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: w.collection.to_string(),
                action: ApprovalAction::ApproveAll {
                    operator: w.marketplace.to_string(),
                    expires: Some(in_a_week(&w)),
                },
            },
            &[],
        )
        .unwrap();
    assert!(is_operator(&w, &w.alice, &w.marketplace));
    w.app
        .execute_contract(
            w.admin.clone(),
            w.proxy.clone(),
            &ExecuteMsg::RemoveAllowedOperator {
                operator: w.marketplace.to_string(),
            },
            &[],
        )
        .unwrap();
    let cfg: xion_asset_proxy::msg::ConfigResponse = w
        .app
        .wrap()
        .query_wasm_smart(&w.proxy, &QueryMsg::Config {})
        .unwrap();
    assert!(cfg.allowed_operators.is_empty());
    // the stored approval survives the removal ...
    assert!(is_operator(&w, &w.alice, &w.marketplace));
    // ... new approvals are refused ...
    let err = w
        .app
        .execute_contract(
            w.buyer.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: w.collection.to_string(),
                action: ApprovalAction::ApproveAll {
                    operator: w.marketplace.to_string(),
                    expires: Some(in_a_week(&w)),
                },
            },
            &[],
        )
        .unwrap_err();
    assert!(
        err.root_cause()
            .to_string()
            .contains("Operator is not allowed"),
        "{err:#}"
    );
    // ... the policy query agrees ...
    let q: xion_asset_proxy::msg::IsAllowedResponse = w
        .app
        .wrap()
        .query_wasm_smart(
            &w.proxy,
            &QueryMsg::IsAllowed {
                collection: w.collection.to_string(),
                action: xion_asset_proxy::msg::ProxyAction::RevokeAll {
                    operator: w.marketplace.to_string(),
                },
            },
        )
        .unwrap();
    assert!(q.allowed);
    // ... a second user's approval to the same operator is untouched by alice's revoke ...
    // (buyer approved directly earlier in this test as operator for alice; give buyer one too)
    w.app
        .execute_contract(
            w.buyer.clone(),
            w.collection.clone(),
            &BaseExecuteMsg::ApproveAll {
                operator: w.marketplace.to_string(),
                expires: None,
            },
            &[],
        )
        .unwrap();
    let revoke = ExecuteMsg::SponsoredApproval {
        collection: w.collection.to_string(),
        action: ApprovalAction::RevokeAll {
            operator: w.marketplace.to_string(),
        },
    };
    w.app
        .execute_contract(w.alice.clone(), w.proxy.clone(), &revoke, &[])
        .unwrap();
    assert!(!is_operator(&w, &w.alice, &w.marketplace));
    assert!(is_operator(&w, &w.buyer, &w.marketplace));
    // ... revoking again is a harmless no-op ...
    w.app
        .execute_contract(w.alice.clone(), w.proxy.clone(), &revoke, &[])
        .unwrap();
    // ... and a malformed operator is rejected by the collection, not silently accepted
    let err = w
        .app
        .execute_contract(
            w.alice.clone(),
            w.proxy.clone(),
            &ExecuteMsg::SponsoredApproval {
                collection: w.collection.to_string(),
                action: ApprovalAction::RevokeAll {
                    operator: "not-an-address".to_string(),
                },
            },
            &[],
        )
        .unwrap_err();
    assert!(!err.root_cause().to_string().is_empty());
    assert!(is_operator(&w, &w.buyer, &w.marketplace));
}

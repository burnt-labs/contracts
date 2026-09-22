//! cw-multi-test coverage: the wrapper through real entrypoints, and migration of a live
//! base `asset` collection onto the variant code id.

use asset_proxyable::msg::{
    BaseExecuteMsg, BaseQueryMsg, InstantiateMsg, MigrateMsg, ProxyAction, ProxyMsg, ProxyQueryMsg,
    ProxyableExecuteMsg, ProxyableQueryMsg,
};
use cosmwasm_std::{Addr, Empty, coin, from_json};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use cw2::ContractVersion;
use cw721::msg::{OperatorResponse, OwnerOfResponse};

fn proxyable_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            asset_proxyable::contract::execute,
            asset_proxyable::contract::instantiate,
            asset_proxyable::contract::query,
        )
        .with_migrate_empty(|deps, env, msg, _info| {
            asset_proxyable::contract::migrate(deps, env, msg)
        }),
    )
}

fn base_asset_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            asset::contracts::asset_base::execute,
            asset::contracts::asset_base::instantiate,
            asset::contracts::asset_base::query,
        )
        .with_migrate_empty(|deps, env, msg, _info| {
            asset::contracts::asset_base::migrate(deps, env, msg)
        }),
    )
}

fn instantiate_msg(creator: &Addr) -> InstantiateMsg {
    InstantiateMsg {
        name: "Variant".to_string(),
        symbol: "VAR".to_string(),
        collection_info_extension: None,
        minter: Some(creator.to_string()),
        creator: Some(creator.to_string()),
        withdraw_address: None,
    }
}

fn mint(app: &mut App, collection: &Addr, minter: &Addr, owner: &Addr, token_id: &str) {
    let msg = BaseExecuteMsg::Mint {
        token_id: token_id.to_string(),
        owner: owner.to_string(),
        token_uri: None,
        extension: None,
    };
    app.execute_contract(minter.clone(), collection.clone(), &msg, &[])
        .unwrap();
}

fn owner_of(app: &App, collection: &Addr, token_id: &str) -> Option<String> {
    app.wrap()
        .query_wasm_smart::<OwnerOfResponse>(
            collection,
            &BaseQueryMsg::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: None,
            },
        )
        .ok()
        .map(|r| r.owner)
}

fn cw2_version(app: &App, collection: &Addr) -> ContractVersion {
    let raw = app
        .wrap()
        .query_wasm_raw(collection, b"contract_info")
        .unwrap()
        .expect("cw2 info present");
    from_json(&raw).unwrap()
}

fn trusted_proxies(app: &App, collection: &Addr) -> Vec<Addr> {
    app.wrap()
        .query_wasm_smart(
            collection,
            &ProxyableQueryMsg::Proxy(ProxyQueryMsg::GetTrustedProxies {}),
        )
        .unwrap()
}

#[test]
fn end_to_end_proxied_flow() {
    let mut app = App::default();
    let creator = app.api().addr_make("creator");
    let alice = app.api().addr_make("alice");
    let proxy = app.api().addr_make("proxy"); // any address can play the proxy here
    let marketplace = app.api().addr_make("marketplace");
    // give the proxy a balance so the funds-attached case reaches the contract
    app.sudo(cw_multi_test::SudoMsg::Bank(
        cw_multi_test::BankSudo::Mint {
            to_address: proxy.to_string(),
            amount: vec![coin(10, "uxion")],
        },
    ))
    .unwrap();

    let code_id = app.store_code(proxyable_contract());
    let collection = app
        .instantiate_contract(
            code_id,
            creator.clone(),
            &instantiate_msg(&creator),
            &[],
            "variant",
            Some(creator.to_string()),
        )
        .unwrap();
    assert_eq!(cw2_version(&app, &collection).contract, "asset-proxyable");
    mint(&mut app, &collection, &creator, &alice, "t1");

    // creator registers the proxy
    app.execute_contract(
        creator.clone(),
        collection.clone(),
        &ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            proxy: proxy.to_string(),
        }),
        &[],
    )
    .unwrap();
    assert_eq!(trusted_proxies(&app, &collection), vec![proxy.clone()]);

    // proxy approves the marketplace on alice's behalf
    app.execute_contract(
        proxy.clone(),
        collection.clone(),
        &ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
            sender: alice.to_string(),
            action: ProxyAction::ApproveAll {
                operator: marketplace.to_string(),
                expires: None,
            },
        }),
        &[],
    )
    .unwrap();
    let op: OperatorResponse = app
        .wrap()
        .query_wasm_smart(
            &collection,
            &BaseQueryMsg::Operator {
                owner: alice.to_string(),
                operator: marketplace.to_string(),
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(op.approval.spender, marketplace);

    // funds attached: rejected before anything runs
    let err = app
        .execute_contract(
            proxy.clone(),
            collection.clone(),
            &ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
                sender: alice.to_string(),
                action: ProxyAction::Burn {
                    token_id: "t1".to_string(),
                },
            }),
            &[coin(1, "uxion")],
        )
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("accept funds"),
        "{err:#}"
    );

    // proxy burns on alice's behalf
    app.execute_contract(
        proxy.clone(),
        collection.clone(),
        &ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
            sender: alice.to_string(),
            action: ProxyAction::Burn {
                token_id: "t1".to_string(),
            },
        }),
        &[],
    )
    .unwrap();
    assert_eq!(owner_of(&app, &collection, "t1"), None);
}

#[test]
fn base_collection_migrates_onto_the_variant() {
    let mut app = App::default();
    let creator = app.api().addr_make("creator");
    let alice = app.api().addr_make("alice");
    let proxy = app.api().addr_make("proxy");

    // a live base `asset` collection with an admin, a token and a listing
    let base_code = app.store_code(base_asset_contract());
    let collection = app
        .instantiate_contract(
            base_code,
            creator.clone(),
            &instantiate_msg(&creator),
            &[],
            "base",
            Some(creator.to_string()),
        )
        .unwrap();
    assert_eq!(cw2_version(&app, &collection).contract, "asset");
    mint(&mut app, &collection, &creator, &alice, "t1");
    app.execute_contract(
        alice.clone(),
        collection.clone(),
        &BaseExecuteMsg::UpdateExtension {
            msg: asset::msg::AssetExtensionExecuteMsg::List {
                token_id: "t1".to_string(),
                price: coin(100, "uxion"),
                reservation: None,
            },
        },
        &[],
    )
    .unwrap();

    // the variant cannot be spoken to yet
    assert!(
        app.wrap()
            .query_wasm_smart::<Vec<Addr>>(
                &collection,
                &ProxyableQueryMsg::Proxy(ProxyQueryMsg::GetTrustedProxies {}),
            )
            .is_err()
    );

    // migrate onto the variant
    let variant_code = app.store_code(proxyable_contract());
    app.migrate_contract(
        creator.clone(),
        collection.clone(),
        &MigrateMsg::WithUpdate {
            minter: None,
            creator: None,
        },
        variant_code,
    )
    .unwrap();

    let v = cw2_version(&app, &collection);
    assert_eq!(v.contract, "asset-proxyable");
    // state intact
    assert_eq!(owner_of(&app, &collection, "t1"), Some(alice.to_string()));
    let listing: asset::state::ListingInfo = app
        .wrap()
        .query_wasm_smart(
            &collection,
            &BaseQueryMsg::Extension {
                msg: asset::msg::AssetExtensionQueryMsg::GetListing {
                    token_id: "t1".to_string(),
                },
            },
        )
        .unwrap();
    assert_eq!(listing.seller, alice);
    assert!(trusted_proxies(&app, &collection).is_empty());

    // and the variant works: register proxy, proxied burn blocked by the listing, delist, burn
    app.execute_contract(
        creator.clone(),
        collection.clone(),
        &ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            proxy: proxy.to_string(),
        }),
        &[],
    )
    .unwrap();
    let burn = ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
        sender: alice.to_string(),
        action: ProxyAction::Burn {
            token_id: "t1".to_string(),
        },
    });
    let err = app
        .execute_contract(proxy.clone(), collection.clone(), &burn, &[])
        .unwrap_err();
    assert!(
        err.root_cause().to_string().contains("while it is listed"),
        "{err:#}"
    );
    app.execute_contract(
        alice.clone(),
        collection.clone(),
        &BaseExecuteMsg::UpdateExtension {
            msg: asset::msg::AssetExtensionExecuteMsg::Delist {
                token_id: "t1".to_string(),
            },
        },
        &[],
    )
    .unwrap();
    app.execute_contract(proxy.clone(), collection.clone(), &burn, &[])
        .unwrap();
    assert_eq!(owner_of(&app, &collection, "t1"), None);

    // a second migration (variant -> variant) keeps the proxy set
    let variant_code_2 = app.store_code(proxyable_contract());
    app.migrate_contract(
        creator.clone(),
        collection.clone(),
        &MigrateMsg::WithUpdate {
            minter: None,
            creator: None,
        },
        variant_code_2,
    )
    .unwrap();
    assert_eq!(trusted_proxies(&app, &collection), vec![proxy]);
}

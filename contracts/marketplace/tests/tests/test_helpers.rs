use anyhow::Error;
use asset::msg::InstantiateMsg as AssetInstantiateMsg;
use cosmwasm_std::{coin, Addr, Empty};
use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
use cw_multi_test::{App, AppResponse, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use serde_json::json;
use xion_nft_marketplace::msg::{ExecuteMsg, InstantiateMsg};

pub fn assert_error(result: Result<AppResponse, Error>, expected: String) {
    assert_eq!(result.unwrap_err().source().unwrap().to_string(), expected);
}

pub fn asset_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        asset::contracts::asset_base::execute,
        asset::contracts::asset_base::instantiate,
        asset::contracts::asset_base::query,
    ))
}

pub fn marketplace_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        xion_nft_marketplace::execute::execute,
        xion_nft_marketplace::contract::instantiate,
        xion_nft_marketplace::query::query,
    ))
}

pub fn setup_app() -> App {
    App::default()
}

pub fn setup_app_with_balances() -> App {
    let mut app = App::default();

    // Create proper addresses using app.api().addr_make
    let buyer = app.api().addr_make("buyer");
    let seller = app.api().addr_make("seller");
    let minter = app.api().addr_make("minter");
    let manager = app.api().addr_make("manager");

    // Mint tokens to test accounts using sudo
    let funds = vec![coin(10000, "uxion")];

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: buyer.to_string(),
        amount: funds.clone(),
    }))
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: seller.to_string(),
        amount: funds.clone(),
    }))
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: minter.to_string(),
        amount: funds.clone(),
    }))
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: manager.to_string(),
        amount: funds.clone(),
    }))
    .unwrap();

    app
}

pub fn setup_asset_contract(app: &mut App, minter: &Addr) -> Addr {
    let asset_code_id = app.store_code(asset_contract());

    let instantiate_msg = AssetInstantiateMsg {
        name: "Test Asset".to_string(),
        symbol: "TEST".to_string(),
        minter: Some(minter.to_string()),
        collection_info_extension: cw721::DefaultOptionalCollectionExtensionMsg::default(),
        creator: Some(minter.to_string()),
        withdraw_address: None,
    };

    app.instantiate_contract(
        asset_code_id,
        minter.clone(),
        &instantiate_msg,
        &[],
        "test-asset",
        None,
    )
    .unwrap()
}

pub fn setup_marketplace_contract(app: &mut App, manager: &Addr) -> Addr {
    let marketplace_code_id = app.store_code(marketplace_contract());

    let config_json = json!({
        "manager": manager.to_string(),
        "fee_recipient": manager.to_string(),
        "sale_approvals": false,
        "fee_bps": 250,
        "listing_denom": "uxion"
    });

    let instantiate_msg = InstantiateMsg {
        config: serde_json::from_value(config_json).unwrap(),
    };

    app.instantiate_contract(
        marketplace_code_id,
        manager.clone(),
        &instantiate_msg,
        &[],
        "test-marketplace",
        None,
    )
    .unwrap()
}

pub fn mint_nft(app: &mut App, asset_contract: &Addr, minter: &Addr, owner: &Addr, token_id: &str) {
    let mint_msg = asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        asset::msg::AssetExtensionExecuteMsg,
    >::Mint {
        token_id: token_id.to_string(),
        owner: owner.to_string(),
        token_uri: Some("https://example.com/metadata.json".to_string()),
        extension: cw721::DefaultOptionalNftExtensionMsg::default(),
    };

    app.execute_contract(minter.clone(), asset_contract.clone(), &mint_msg, &[])
        .unwrap();
}

// Reusable function to create a listing and return the listing ID
pub fn create_listing(
    app: &mut App,
    marketplace_contract: &Addr,
    asset_contract: &Addr,
    seller: &Addr,
    token_id: &str,
    price: cosmwasm_std::Coin,
) -> String {
    // Approve marketplace contract to manage the NFT
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price,
        token_id: token_id.to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());

    // Extract listing ID from events
    let events = result.unwrap().events;
    let list_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
        .unwrap();
    // Find the id attribute by key
    let id_attr = list_event
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap();
    id_attr.value.clone()
}

pub fn extract_listing_id_from_events(events: &[cosmwasm_std::Event]) -> String {
    let list_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
        .unwrap();
    list_event
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone()
}

pub fn create_listing_for_buy_test(
    app: &mut App,
    marketplace_contract: &Addr,
    asset_contract: &Addr,
    seller: &Addr,
    token_id: &str,
    price: cosmwasm_std::Coin,
) -> String {
    // Approve marketplace contract to manage the NFT
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price,
        token_id: token_id.to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());

    // Extract listing ID from events
    let events = result.unwrap().events;
    let list_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
        .unwrap();
    let id_attr = list_event
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap();
    id_attr.value.clone()
}

pub fn create_offer_helper(
    app: &mut App,
    marketplace_contract: &Addr,
    asset_contract: &Addr,
    buyer: &Addr,
    token_id: &str,
    price: cosmwasm_std::Coin,
) -> String {
    let offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: token_id.to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &offer_msg,
        &[price],
    );
    assert!(result.is_ok());

    // Extract offer ID from events
    let events = result.unwrap().events;
    let offer_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer")
        .unwrap();
    offer_event
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone()
}

pub fn create_collection_offer_helper(
    app: &mut App,
    marketplace_contract: &Addr,
    asset_contract: &Addr,
    buyer: &Addr,
    price: cosmwasm_std::Coin,
) -> String {
    let offer_msg = ExecuteMsg::CreateCollectionOffer {
        collection: asset_contract.to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &offer_msg,
        &[price],
    );
    assert!(result.is_ok());

    // Extract collection offer ID from events
    let events = result.unwrap().events;
    let offer_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-collection-offer")
        .unwrap();
    offer_event
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone()
}

pub fn setup_marketplace_with_approvals(app: &mut App, manager: &Addr) -> Addr {
    let marketplace_code_id = app.store_code(marketplace_contract());

    let config_json = json!({
        "manager": manager.to_string(),
        "fee_recipient": manager.to_string(),
        "sale_approvals": true,  // Enable approval flow
        "fee_bps": 250,
        "listing_denom": "uxion"
    });

    let instantiate_msg = InstantiateMsg {
        config: serde_json::from_value(config_json).unwrap(),
    };

    app.instantiate_contract(
        marketplace_code_id,
        manager.clone(),
        &instantiate_msg,
        &[],
        "test-marketplace-approvals",
        None,
    )
    .unwrap()
}

pub fn create_listing_helper(
    app: &mut App,
    marketplace_contract: &Addr,
    asset_contract: &Addr,
    seller: &Addr,
    token_id: &str,
    price: cosmwasm_std::Coin,
) -> String {
    // Approve marketplace contract
    let approve_msg = Cw721ExecuteMsg::Approve {
        spender: marketplace_contract.to_string(),
        token_id: token_id.to_string(),
        expires: None,
    };
    app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
        .unwrap();

    let list_msg = ExecuteMsg::ListItem {
        collection: asset_contract.to_string(),
        price,
        token_id: token_id.to_string(),
        reserved_for: None,
    };

    let result = app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
    assert!(result.is_ok());

    // Extract listing ID from events
    let events = result.unwrap().events;
    events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id")
        .unwrap()
        .value
        .clone()
}

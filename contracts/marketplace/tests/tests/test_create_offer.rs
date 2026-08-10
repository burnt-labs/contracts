use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw_multi_test::Executor;
use xion_nft_marketplace::msg::ExecuteMsg;

#[test]
fn test_create_offer_success() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &offer_msg,
        std::slice::from_ref(&price),
    );

    assert!(result.is_ok());

    let response = result.unwrap();
    let events = response.events;
    let offer_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer");
    assert!(offer_event.is_some());

    let id_attr = offer_event
        .unwrap()
        .attributes
        .iter()
        .find(|a| a.key == "id");
    assert!(id_attr.is_some());
}

#[test]
fn test_create_offer_without_funds() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(buyer.clone(), marketplace_contract.clone(), &offer_msg, &[]);

    assert!(result.is_err());
}

#[test]
fn test_create_offer_wrong_denom() {
    let mut app = setup_app();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    use cw_multi_test::{BankSudo, SudoMsg};
    let funds = vec![coin(10000, "fakexion")];
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: buyer.to_string(),
        amount: funds,
    }))
    .unwrap();

    let price = coin(100, "fakexion");
    let offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &offer_msg,
        std::slice::from_ref(&price),
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidListingDenom {
            expected: "uxion".to_string(),
            actual: "fakexion".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_create_offer_insufficient_funds() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let seller = app.api().addr_make("seller");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

    let price = coin(100, "uxion");
    let insufficient = coin(50, "uxion");
    let offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "token1".to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &offer_msg,
        std::slice::from_ref(&insufficient),
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::InvalidPayment {
            expected: price,
            actual: insufficient,
        }
        .to_string(),
    );
}

#[test]
fn test_create_offer_nonexistent_token() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    let price = coin(100, "uxion");
    let offer_msg = ExecuteMsg::CreateOffer {
        collection: asset_contract.to_string(),
        token_id: "nonexistent".to_string(),
        price: price.clone(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &offer_msg,
        std::slice::from_ref(&price),
    );

    assert!(result.is_ok());
}

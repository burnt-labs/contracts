use crate::tests::test_helpers::*;
use cosmwasm_std::coin;
use cw_multi_test::Executor;
use xion_nft_marketplace::msg::ExecuteMsg;

#[test]
fn test_cancel_collection_offer_success() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    let buyer_balance_before = app.wrap().query_balance(buyer.clone(), "uxion").unwrap();

    let price = coin(100, "uxion");
    let offer_id = create_collection_offer_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &buyer,
        price.clone(),
    );

    let cancel_msg = ExecuteMsg::CancelCollectionOffer { id: offer_id };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &cancel_msg,
        &[],
    );

    assert!(result.is_ok());

    let buyer_balance_after = app.wrap().query_balance(buyer.clone(), "uxion").unwrap();
    assert_eq!(buyer_balance_before, buyer_balance_after);

    let events = result.unwrap().events;
    let cancel_event = events
        .iter()
        .find(|e| e.ty == "wasm-xion-nft-marketplace/cancel-collection-offer");
    assert!(cancel_event.is_some());
}

#[test]
fn test_cancel_collection_offer_unauthorized() {
    let mut app = setup_app_with_balances();
    let minter = app.api().addr_make("minter");
    let buyer = app.api().addr_make("buyer");
    let unauthorized = app.api().addr_make("unauthorized");
    let manager = app.api().addr_make("manager");

    let asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    let price = coin(100, "uxion");
    let offer_id = create_collection_offer_helper(
        &mut app,
        &marketplace_contract,
        &asset_contract,
        &buyer,
        price.clone(),
    );

    let cancel_msg = ExecuteMsg::CancelCollectionOffer { id: offer_id };

    let result = app.execute_contract(
        unauthorized.clone(),
        marketplace_contract.clone(),
        &cancel_msg,
        &[],
    );

    assert!(result.is_err());
    assert_error(
        result,
        xion_nft_marketplace::error::ContractError::Unauthorized {
            message: "sender is not the buyer".to_string(),
        }
        .to_string(),
    );
}

#[test]
fn test_cancel_collection_offer_nonexistent() {
    let mut app = setup_app_with_balances();
    let buyer = app.api().addr_make("buyer");
    let manager = app.api().addr_make("manager");
    let minter = app.api().addr_make("minter");

    let _asset_contract = setup_asset_contract(&mut app, &minter);
    let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

    let cancel_msg = ExecuteMsg::CancelCollectionOffer {
        id: "nonexistent-collection-offer".to_string(),
    };

    let result = app.execute_contract(
        buyer.clone(),
        marketplace_contract.clone(),
        &cancel_msg,
        &[],
    );

    assert!(result.is_err());
}

use cosmwasm_std::{coins, Addr, Decimal};
use cw_multi_test::{App, BankSudo, ContractWrapper, Executor, SudoMsg};
use one_way_swap::msg::{ExecuteMsg, InstantiateMsg};
use std::str::FromStr;

fn mint(app: &mut App, to: &Addr, amount: u128, denom: &str) {
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        to_address: to.to_string(),
        amount: coins(amount, denom),
    }))
    .unwrap();
}

fn balance(app: &App, addr: &Addr, denom: &str) -> u128 {
    app.wrap().query_balance(addr, denom).unwrap().amount.u128()
}

#[test]
fn swap_moves_balances_then_fails_without_liquidity() {
    let mut app = App::default();
    let admin = app.api().addr_make("admin");
    let user = app.api().addr_make("user");
    mint(&mut app, &user, 1_000_000, "uusdc");
    mint(&mut app, &admin, 2_000_000, "uverona");

    let code_id = app.store_code(Box::new(ContractWrapper::new_with_empty(
        one_way_swap::contract::execute,
        one_way_swap::contract::instantiate,
        one_way_swap::contract::query,
    )));
    let contract = app
        .instantiate_contract(
            code_id,
            admin.clone(),
            &InstantiateMsg {
                in_denom: "uusdc".into(),
                out_denom: "uverona".into(),
                rate: Decimal::from_str("1.5").unwrap(),
            },
            &[],
            "swap",
            None,
        )
        .unwrap();

    app.send_tokens(
        admin.clone(),
        contract.clone(),
        &coins(1_200_000, "uverona"),
    )
    .unwrap();

    app.execute_contract(
        user.clone(),
        contract.clone(),
        &ExecuteMsg::Swap { min_out: None },
        &coins(500_000, "uusdc"),
    )
    .unwrap();
    assert_eq!(balance(&app, &user, "uverona"), 750_000);
    assert_eq!(balance(&app, &user, "uusdc"), 500_000);
    assert_eq!(balance(&app, &contract, "uusdc"), 500_000);
    assert_eq!(balance(&app, &contract, "uverona"), 450_000);

    // Needs 750_000 uverona but the contract only holds 450_000: whole tx reverts.
    let res = app.execute_contract(
        user.clone(),
        contract.clone(),
        &ExecuteMsg::Swap { min_out: None },
        &coins(500_000, "uusdc"),
    );
    assert!(res.is_err());
    assert_eq!(balance(&app, &user, "uusdc"), 500_000);
    assert_eq!(balance(&app, &user, "uverona"), 750_000);

    app.execute_contract(
        admin.clone(),
        contract.clone(),
        &ExecuteMsg::Withdraw {
            denom: "uusdc".into(),
            amount: None,
        },
        &[],
    )
    .unwrap();
    assert_eq!(balance(&app, &admin, "uusdc"), 500_000);
    assert_eq!(balance(&app, &contract, "uusdc"), 0);
}

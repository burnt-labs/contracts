use crate::error::{ContractError, ContractResult};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use crate::{CONTRACT_NAME, CONTRACT_VERSION};
use cosmwasm_std::{
    coins, ensure, ensure_eq, to_json_binary, BankMsg, Binary, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, Uint128,
};
use cw_utils::{must_pay, nonpayable};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let valid = |d: &str| !d.is_empty() && !d.contains(char::is_whitespace);
    ensure!(
        valid(&msg.in_denom) && valid(&msg.out_denom),
        ContractError::InvalidDenom
    );
    ensure!(msg.in_denom != msg.out_denom, ContractError::SameDenom);
    ensure!(!msg.rate.is_zero(), ContractError::ZeroRate);
    CONFIG.save(
        deps.storage,
        &Config {
            admin: info.sender.clone(),
            pending_admin: None,
            in_denom: msg.in_denom,
            out_denom: msg.out_denom,
            rate: msg.rate,
        },
    )?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", info.sender))
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    cw2::ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("method", "migrate"))
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    match msg {
        ExecuteMsg::Swap { min_out } => {
            let in_amount = must_pay(&info, &config.in_denom)?;
            let out = out_amount(in_amount, config.rate)?;
            let min_out = min_out.unwrap_or_default();
            ensure!(out >= min_out, ContractError::MinOutNotMet { out, min_out });
            Ok(Response::new()
                .add_message(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: coins(out.u128(), &config.out_denom),
                })
                .add_attribute("method", "swap")
                .add_attribute("sender", info.sender)
                .add_attribute("in_amount", in_amount)
                .add_attribute("out_amount", out))
        }
        ExecuteMsg::UpdateRate { rate } => {
            only_admin(&config, &info)?;
            ensure!(!rate.is_zero(), ContractError::ZeroRate);
            config.rate = rate;
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::new()
                .add_attribute("method", "update_rate")
                .add_attribute("rate", rate.to_string()))
        }
        ExecuteMsg::ProposeAdmin { admin } => {
            only_admin(&config, &info)?;
            let pending = deps.api.addr_validate(&admin)?;
            config.pending_admin = Some(pending.clone());
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::new()
                .add_attribute("method", "propose_admin")
                .add_attribute("pending_admin", pending))
        }
        ExecuteMsg::AcceptAdmin {} => {
            nonpayable(&info)?;
            ensure!(
                config.pending_admin.as_ref() == Some(&info.sender),
                ContractError::Unauthorized
            );
            config.admin = info.sender.clone();
            config.pending_admin = None;
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::new()
                .add_attribute("method", "accept_admin")
                .add_attribute("admin", info.sender))
        }
        ExecuteMsg::Withdraw { denom, amount } => {
            only_admin(&config, &info)?;
            let amount = match amount {
                Some(a) => a,
                None => {
                    deps.querier
                        .query_balance(&env.contract.address, &denom)?
                        .amount
                }
            };
            Ok(Response::new()
                .add_message(BankMsg::Send {
                    to_address: info.sender.to_string(),
                    amount: coins(amount.u128(), &denom),
                })
                .add_attribute("method", "withdraw")
                .add_attribute("denom", denom)
                .add_attribute("amount", amount))
        }
    }
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&CONFIG.load(deps.storage)?)?),
        QueryMsg::Simulate { amount } => Ok(to_json_binary(&out_amount(
            amount,
            CONFIG.load(deps.storage)?.rate,
        )?)?),
    }
}

fn only_admin(config: &Config, info: &MessageInfo) -> ContractResult<()> {
    nonpayable(info)?;
    ensure_eq!(info.sender, config.admin, ContractError::Unauthorized);
    Ok(())
}

fn out_amount(in_amount: Uint128, rate: Decimal) -> ContractResult<Uint128> {
    let out = in_amount.checked_mul_floor(rate)?;
    ensure!(!out.is_zero(), ContractError::SwapTooSmall);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        message_info, mock_dependencies, mock_dependencies_with_balance, mock_env, MockApi,
        MockQuerier, MockStorage,
    };
    use cosmwasm_std::{from_json, Addr, Coin, CosmosMsg, OwnedDeps};
    use cw_utils::PaymentError;
    use std::str::FromStr;

    type TestDeps = OwnedDeps<MockStorage, MockApi, MockQuerier>;

    fn init(deps: &mut TestDeps, rate: &str) -> Addr {
        let admin = deps.api.addr_make("admin");
        instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&admin, &[]),
            InstantiateMsg {
                in_denom: "uusdc".into(),
                out_denom: "uverona".into(),
                rate: Decimal::from_str(rate).unwrap(),
            },
        )
        .unwrap();
        admin
    }

    fn setup(rate: &str) -> (TestDeps, Addr) {
        let mut deps = mock_dependencies();
        let admin = init(&mut deps, rate);
        (deps, admin)
    }

    fn exec(
        deps: &mut TestDeps,
        sender: &Addr,
        funds: &[Coin],
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        execute(deps.as_mut(), mock_env(), message_info(sender, funds), msg)
    }

    fn bank_send(to: &Addr, amount: Vec<Coin>) -> CosmosMsg {
        CosmosMsg::Bank(BankMsg::Send {
            to_address: to.to_string(),
            amount,
        })
    }

    #[test]
    fn instantiate_rejects_bad_config() {
        let mut deps = mock_dependencies();
        let admin = deps.api.addr_make("admin");
        let cases = [
            ("uusdc", "uverona", "0", ContractError::ZeroRate),
            ("uusdc", "uusdc", "1", ContractError::SameDenom),
            ("uusdc", "", "1", ContractError::InvalidDenom),
            ("uusdc ", "uverona", "1", ContractError::InvalidDenom),
        ];
        for (in_denom, out_denom, rate, expected) in cases {
            let err = instantiate(
                deps.as_mut(),
                mock_env(),
                message_info(&admin, &[]),
                InstantiateMsg {
                    in_denom: in_denom.into(),
                    out_denom: out_denom.into(),
                    rate: Decimal::from_str(rate).unwrap(),
                },
            )
            .unwrap_err();
            assert_eq!(err.to_string(), expected.to_string());
        }
    }

    #[test]
    fn swap_pays_floor_of_amount_times_rate() {
        let (mut deps, _) = setup("2.5");
        let user = deps.api.addr_make("user");
        let res = exec(
            &mut deps,
            &user,
            &coins(1_000_001, "uusdc"),
            ExecuteMsg::Swap { min_out: None },
        )
        .unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(
            res.messages[0].msg,
            bank_send(&user, coins(2_500_002, "uverona"))
        );
    }

    #[test]
    fn swap_rejects_wrong_denom_and_dust() {
        let (mut deps, _) = setup("1");
        let user = deps.api.addr_make("user");
        let err = exec(
            &mut deps,
            &user,
            &coins(100, "uverona"),
            ExecuteMsg::Swap { min_out: None },
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::Payment(PaymentError::MissingDenom(_))),
            "got {err:?}"
        );

        let (mut deps, _) = setup("0.000001");
        let err = exec(
            &mut deps,
            &user,
            &coins(1, "uusdc"),
            ExecuteMsg::Swap { min_out: None },
        )
        .unwrap_err();
        assert!(matches!(err, ContractError::SwapTooSmall), "got {err:?}");
    }

    #[test]
    fn swap_enforces_min_out() {
        let (mut deps, _) = setup("2.5");
        let user = deps.api.addr_make("user");
        let swap = |min_out| ExecuteMsg::Swap {
            min_out: Some(Uint128::new(min_out)),
        };
        let funds = coins(1_000_001, "uusdc");
        let err = exec(&mut deps, &user, &funds, swap(2_500_003)).unwrap_err();
        assert!(
            matches!(err, ContractError::MinOutNotMet { .. }),
            "got {err:?}"
        );
        exec(&mut deps, &user, &funds, swap(2_500_002)).unwrap();
    }

    #[test]
    fn admin_ops_reject_non_admin() {
        let (mut deps, admin) = setup("1");
        let user = deps.api.addr_make("user");
        let msgs = [
            ExecuteMsg::UpdateRate {
                rate: Decimal::one(),
            },
            ExecuteMsg::ProposeAdmin {
                admin: user.to_string(),
            },
            ExecuteMsg::AcceptAdmin {},
            ExecuteMsg::Withdraw {
                denom: "uusdc".into(),
                amount: None,
            },
        ];
        for msg in msgs {
            let err = exec(&mut deps, &user, &[], msg).unwrap_err();
            assert!(matches!(err, ContractError::Unauthorized), "got {err:?}");
        }

        let err = exec(
            &mut deps,
            &admin,
            &coins(1, "uusdc"),
            ExecuteMsg::UpdateRate {
                rate: Decimal::one(),
            },
        )
        .unwrap_err();
        assert!(
            matches!(err, ContractError::Payment(PaymentError::NonPayable {})),
            "got {err:?}"
        );
    }

    #[test]
    fn admin_handoff_requires_accept() {
        let (mut deps, admin) = setup("1");
        let new_admin = deps.api.addr_make("new_admin");
        let user = deps.api.addr_make("user");
        let rate = |r: &str| ExecuteMsg::UpdateRate {
            rate: Decimal::from_str(r).unwrap(),
        };
        let unauthorized =
            |res: ContractResult<Response>| matches!(res, Err(ContractError::Unauthorized));

        exec(
            &mut deps,
            &admin,
            &[],
            ExecuteMsg::ProposeAdmin {
                admin: new_admin.to_string(),
            },
        )
        .unwrap();
        // Old admin keeps control until the nominee accepts.
        assert!(unauthorized(exec(&mut deps, &new_admin, &[], rate("2"))));
        exec(&mut deps, &admin, &[], rate("2")).unwrap();
        assert!(unauthorized(exec(
            &mut deps,
            &user,
            &[],
            ExecuteMsg::AcceptAdmin {}
        )));

        exec(&mut deps, &new_admin, &[], ExecuteMsg::AcceptAdmin {}).unwrap();
        assert!(unauthorized(exec(&mut deps, &admin, &[], rate("3"))));
        exec(&mut deps, &new_admin, &[], rate("3")).unwrap();

        let config: Config =
            from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.admin, new_admin);
        assert_eq!(config.pending_admin, None);
    }

    #[test]
    fn update_rate_changes_simulate() {
        let (mut deps, admin) = setup("1");
        exec(
            &mut deps,
            &admin,
            &[],
            ExecuteMsg::UpdateRate {
                rate: Decimal::from_str("3").unwrap(),
            },
        )
        .unwrap();
        let out: Uint128 = from_json(
            query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::Simulate {
                    amount: Uint128::new(10),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(out, Uint128::new(30));
    }

    #[test]
    fn withdraw_without_amount_sends_full_balance() {
        let mut deps = mock_dependencies_with_balance(&coins(500, "uusdc"));
        let admin = init(&mut deps, "1");

        let res = exec(
            &mut deps,
            &admin,
            &[],
            ExecuteMsg::Withdraw {
                denom: "uusdc".into(),
                amount: None,
            },
        )
        .unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0].msg, bank_send(&admin, coins(500, "uusdc")));

        let res = exec(
            &mut deps,
            &admin,
            &[],
            ExecuteMsg::Withdraw {
                denom: "uusdc".into(),
                amount: Some(Uint128::new(200)),
            },
        )
        .unwrap();
        assert_eq!(res.messages.len(), 1);
        assert_eq!(res.messages[0].msg, bank_send(&admin, coins(200, "uusdc")));
    }

    #[test]
    fn migrate_only_from_older_same_contract() {
        let (mut deps, _) = setup("1");
        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "0.0.1").unwrap();
        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();
        let version = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(version.version, CONTRACT_VERSION);

        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "9.9.9").unwrap();
        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap_err();

        cw2::set_contract_version(deps.as_mut().storage, "user-map", "0.0.1").unwrap();
        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap_err();
    }
}

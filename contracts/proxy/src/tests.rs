use asset_proxyable::msg::{ProxyMsg, ProxyableExecuteMsg};
use cosmwasm_std::{
    Addr, Coin, CosmosMsg, DepsMut, OwnedDeps, Response, Timestamp, WasmMsg, coin, from_json,
    testing::{MockApi, MockQuerier, MockStorage, message_info, mock_dependencies, mock_env},
};
use cw721::Expiration;

use crate::{
    CONTRACT_NAME,
    contract::{execute, instantiate, query},
    error::ContractError,
    msg::{
        ApprovalAction, ConfigResponse, ExecuteMsg, InstantiateMsg, IsAllowedResponse, ProxyAction,
        QueryMsg,
    },
    state::count_collections,
    state::{COLLECTIONS, MAX_COLLECTIONS},
};

type Deps = OwnedDeps<MockStorage, MockApi, MockQuerier>;

struct Actors {
    admin: Addr,
    alice: Addr,
    bob: Addr,
    marketplace: Addr,
    other_operator: Addr,
    good: Addr,
    good2: Addr,
}

fn actors(deps: &Deps) -> Actors {
    Actors {
        admin: deps.api.addr_make("admin"),
        alice: deps.api.addr_make("alice"),
        bob: deps.api.addr_make("bob"),
        marketplace: deps.api.addr_make("marketplace"),
        other_operator: deps.api.addr_make("other_operator"),
        good: deps.api.addr_make("good_collection"),
        good2: deps.api.addr_make("good_collection_2"),
    }
}

fn default_msg(a: &Actors, max_approval_seconds: Option<u64>) -> InstantiateMsg {
    InstantiateMsg {
        admin: a.admin.to_string(),
        allowed_operators: vec![a.marketplace.to_string()],
        max_approval_seconds,
        collections: vec![a.good.to_string()],
    }
}

fn setup(max_approval_seconds: Option<u64>) -> (Deps, Actors) {
    let mut deps = mock_dependencies();
    let a = actors(&deps);
    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&a.admin, &[]),
        default_msg(&a, max_approval_seconds),
    )
    .unwrap();
    (deps, a)
}

fn exec(
    deps: DepsMut,
    sender: &Addr,
    funds: &[Coin],
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    execute(deps, mock_env(), message_info(sender, funds), msg)
}

fn approve(collection: &Addr, operator: &Addr, expires: Option<Expiration>) -> ExecuteMsg {
    ExecuteMsg::SponsoredApproval {
        collection: collection.to_string(),
        action: ApprovalAction::ApproveAll {
            operator: operator.to_string(),
            expires,
        },
    }
}

fn burn(collection: &Addr, token_id: &str) -> ExecuteMsg {
    ExecuteMsg::SponsoredBurn {
        collection: collection.to_string(),
        token_id: token_id.to_string(),
    }
}

/// Extract the single forwarded envelope from a response.
fn forwarded(r: &Response) -> (String, ProxyableExecuteMsg, Vec<Coin>) {
    assert_eq!(r.messages.len(), 1, "exactly one forwarded message");
    match &r.messages[0].msg {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg,
            funds,
        }) => (
            contract_addr.clone(),
            from_json(msg).unwrap(),
            funds.clone(),
        ),
        other => panic!("unexpected message {other:?}"),
    }
}

fn is_allowed(deps: &Deps, collection: &Addr, action: ProxyAction) -> IsAllowedResponse {
    from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::IsAllowed {
                collection: collection.to_string(),
                action,
            },
        )
        .unwrap(),
    )
    .unwrap()
}

fn config(deps: &Deps) -> ConfigResponse {
    from_json(query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap()
}

fn collections(deps: &Deps, start_after: Option<&Addr>, limit: Option<u32>) -> Vec<Addr> {
    from_json(
        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Collections {
                start_after: start_after.map(|a| a.to_string()),
                limit,
            },
        )
        .unwrap(),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------------------
// instantiate
// ---------------------------------------------------------------------------------------

#[test]
fn instantiate_validates_and_records_config() {
    let (deps, a) = setup(Some(3600));
    let v = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(v.contract, CONTRACT_NAME);
    let c = config(&deps);
    assert_eq!(c.admin, a.admin);
    assert_eq!(c.allowed_operators, vec![a.marketplace.clone()]);
    assert_eq!(c.max_approval_seconds, Some(3600));
    assert_eq!(collections(&deps, None, None), vec![a.good.clone()]);
}

#[test]
fn instantiate_rejects_bad_config() {
    let mut deps = mock_dependencies();
    let a = actors(&deps);
    let run = |deps: &mut Deps, msg: InstantiateMsg| {
        instantiate(deps.as_mut(), mock_env(), message_info(&a.admin, &[]), msg)
    };

    let mut m = default_msg(&a, None);
    m.allowed_operators = vec![];
    assert!(matches!(
        run(&mut deps, m).unwrap_err(),
        ContractError::InvalidConfig { .. }
    ));

    let mut m = default_msg(&a, None);
    m.allowed_operators = (0..5)
        .map(|i| deps.api.addr_make(&format!("op{i}")).to_string())
        .collect();
    assert_eq!(
        run(&mut deps, m).unwrap_err(),
        ContractError::TooManyOperators { max: 4 }
    );

    // approval cap must be sane; a huge cap would make the expiry math meaningless
    for cap in [0u64, crate::state::MAX_APPROVAL_CAP_SECONDS + 1, u64::MAX] {
        let m = default_msg(&a, Some(cap));
        assert!(
            matches!(
                run(&mut deps, m).unwrap_err(),
                ContractError::InvalidConfig { .. }
            ),
            "cap {cap}"
        );
    }
    // the collection cap applies at instantiation too
    let mut m = default_msg(&a, None);
    m.collections = (0..=MAX_COLLECTIONS)
        .map(|i| deps.api.addr_make(&format!("many{i}")).to_string())
        .collect();
    assert_eq!(
        run(&mut deps, m).unwrap_err(),
        ContractError::TooManyCollections {
            max: MAX_COLLECTIONS
        }
    );

    // the proxy cannot allowlist itself
    let mut m = default_msg(&a, None);
    m.collections = vec![mock_env().contract.address.to_string()];
    assert_eq!(run(&mut deps, m).unwrap_err(), ContractError::SelfTarget {});

    // nor administer itself (reachable via Instantiate2 with a precomputed address)
    let mut m = default_msg(&a, None);
    m.admin = mock_env().contract.address.to_string();
    assert_eq!(run(&mut deps, m).unwrap_err(), ContractError::SelfTarget {});

    // funds rejected
    assert!(matches!(
        instantiate(
            deps.as_mut(),
            mock_env(),
            message_info(&a.admin, &[coin(1, "uxion")]),
            default_msg(&a, None)
        )
        .unwrap_err(),
        ContractError::Payment(_)
    ));
}

// ---------------------------------------------------------------------------------------
// sponsored actions
// ---------------------------------------------------------------------------------------

#[test]
fn sponsored_burn_forwards_the_caller_as_sender() {
    let (mut deps, a) = setup(None);
    let r = exec(deps.as_mut(), &a.alice, &[], burn(&a.good, "t1")).unwrap();
    let (target, envelope, funds) = forwarded(&r);
    assert_eq!(target, a.good.to_string());
    assert!(funds.is_empty());
    assert_eq!(
        envelope,
        ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
            sender: a.alice.to_string(),
            action: ProxyAction::Burn {
                token_id: "t1".to_string()
            },
        })
    );
    assert!(
        r.attributes
            .iter()
            .any(|x| x.key == "action" && x.value == "sponsored_burn")
    );
    // the JSON shape the collection expects
    let raw = serde_json::to_value(&envelope).unwrap();
    assert!(raw.get("proxy_execute").is_some());
}

#[test]
fn sponsored_approval_forwards_and_enforces_operator_policy() {
    let (mut deps, a) = setup(None);
    let r = exec(
        deps.as_mut(),
        &a.alice,
        &[],
        approve(&a.good, &a.marketplace, None),
    )
    .unwrap();
    let (_, envelope, _) = forwarded(&r);
    assert_eq!(
        envelope,
        ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
            sender: a.alice.to_string(),
            action: ProxyAction::ApproveAll {
                operator: a.marketplace.to_string(),
                expires: None
            },
        })
    );

    // an operator outside the allowlist is the whole point of the policy
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.alice,
            &[],
            approve(&a.good, &a.other_operator, None)
        )
        .unwrap_err(),
        ContractError::OperatorNotAllowed {
            operator: a.other_operator.to_string()
        }
    );
    // revoke is bounded to operators this proxy ever allowed: an operator the user approved
    // elsewhere is none of the proxy's business, so a stolen session cannot disturb it
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.alice,
            &[],
            ExecuteMsg::SponsoredApproval {
                collection: a.good.to_string(),
                action: ApprovalAction::RevokeAll {
                    operator: a.other_operator.to_string(),
                },
            },
        )
        .unwrap_err(),
        ContractError::OperatorNotAllowed {
            operator: a.other_operator.to_string()
        }
    );
    assert!(
        !is_allowed(
            &deps,
            &a.good,
            ProxyAction::RevokeAll {
                operator: a.other_operator.to_string()
            }
        )
        .allowed
    );
    // but revoking the configured operator forwards fine
    let r = exec(
        deps.as_mut(),
        &a.alice,
        &[],
        ExecuteMsg::SponsoredApproval {
            collection: a.good.to_string(),
            action: ApprovalAction::RevokeAll {
                operator: a.marketplace.to_string(),
            },
        },
    )
    .unwrap();
    let (_, envelope, _) = forwarded(&r);
    assert_eq!(
        envelope,
        ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
            sender: a.alice.to_string(),
            action: ProxyAction::RevokeAll {
                operator: a.marketplace.to_string()
            },
        })
    );
    // an unvalidated spelling of an allowed operator does not match the stored key
    let shouty = a.marketplace.to_string().to_uppercase();
    assert!(matches!(
        exec(
            deps.as_mut(),
            &a.alice,
            &[],
            ExecuteMsg::SponsoredApproval {
                collection: a.good.to_string(),
                action: ApprovalAction::ApproveAll {
                    operator: shouty,
                    expires: None
                },
            }
        )
        .unwrap_err(),
        ContractError::OperatorNotAllowed { .. }
    ));
}

#[test]
fn approval_expiry_cap_is_enforced() {
    let (mut deps, a) = setup(Some(3600));
    let now = mock_env().block.time;
    let cases: Vec<(Option<Expiration>, bool)> = vec![
        (None, false),
        (Some(Expiration::Never {}), false),
        (Some(Expiration::AtHeight(10_000_000)), false),
        (Some(Expiration::AtTime(now.plus_seconds(3601))), false),
        (Some(Expiration::AtTime(now)), false),
        (Some(Expiration::AtTime(now.minus_seconds(1))), false),
        (Some(Expiration::AtTime(now.plus_seconds(3600))), true),
        (Some(Expiration::AtTime(now.plus_seconds(1))), true),
    ];
    for (expires, ok) in cases {
        let res = exec(
            deps.as_mut(),
            &a.alice,
            &[],
            approve(&a.good, &a.marketplace, expires),
        );
        assert_eq!(res.is_ok(), ok, "expires={expires:?}");
        if !ok {
            assert_eq!(
                res.unwrap_err(),
                ContractError::ApprovalExpiryOutOfBounds { max_seconds: 3600 }
            );
        }
        // the query mirrors the execute decision exactly
        let q = is_allowed(
            &deps,
            &a.good,
            ProxyAction::ApproveAll {
                operator: a.marketplace.to_string(),
                expires,
            },
        );
        assert_eq!(q.allowed, ok, "query mismatch for {expires:?}");
    }
    // revoke has no expiry and is unaffected by the cap
    exec(
        deps.as_mut(),
        &a.alice,
        &[],
        ExecuteMsg::SponsoredApproval {
            collection: a.good.to_string(),
            action: ApprovalAction::RevokeAll {
                operator: a.marketplace.to_string(),
            },
        },
    )
    .unwrap();
    // without a cap, anything goes (cw721 then defaults None to Never)
    let (mut deps, a) = setup(None);
    exec(
        deps.as_mut(),
        &a.alice,
        &[],
        approve(&a.good, &a.marketplace, Some(Expiration::Never {})),
    )
    .unwrap();
}

#[test]
fn only_allowlisted_collections_and_never_self() {
    let (mut deps, a) = setup(None);
    assert_eq!(
        exec(deps.as_mut(), &a.alice, &[], burn(&a.good2, "t1")).unwrap_err(),
        ContractError::CollectionNotAllowed {
            collection: a.good2.to_string()
        }
    );
    let me = mock_env().contract.address;
    assert_eq!(
        exec(deps.as_mut(), &a.alice, &[], burn(&me, "t1")).unwrap_err(),
        ContractError::SelfTarget {}
    );
    let q = is_allowed(
        &deps,
        &a.good2,
        ProxyAction::Burn {
            token_id: "t1".to_string(),
        },
    );
    assert!(!q.allowed);
    assert!(q.reason.unwrap().contains("not allowlisted"));
}

#[test]
fn nothing_is_payable() {
    let (mut deps, a) = setup(None);
    let msgs = vec![
        burn(&a.good, "t1"),
        approve(&a.good, &a.marketplace, None),
        ExecuteMsg::AddCollection {
            collection: a.good2.to_string(),
        },
        ExecuteMsg::RemoveCollection {
            collection: a.good.to_string(),
        },
        ExecuteMsg::RemoveAllowedOperator {
            operator: a.marketplace.to_string(),
        },
        ExecuteMsg::UpdateAdmin {
            admin: a.bob.to_string(),
        },
    ];
    for m in msgs {
        let err = exec(deps.as_mut(), &a.admin, &[coin(1, "uxion")], m.clone()).unwrap_err();
        assert!(matches!(err, ContractError::Payment(_)), "{m:?} -> {err:?}");
    }
}

// ---------------------------------------------------------------------------------------
// admin
// ---------------------------------------------------------------------------------------

#[test]
fn admin_manages_collections() {
    let (mut deps, a) = setup(None);
    let add = |c: &Addr| ExecuteMsg::AddCollection {
        collection: c.to_string(),
    };
    let remove = |c: &Addr| ExecuteMsg::RemoveCollection {
        collection: c.to_string(),
    };

    assert_eq!(
        exec(deps.as_mut(), &a.bob, &[], add(&a.good2)).unwrap_err(),
        ContractError::Unauthorized {}
    );
    assert_eq!(
        exec(deps.as_mut(), &a.admin, &[], add(&a.good)).unwrap_err(),
        ContractError::CollectionAlreadyAllowed {
            collection: a.good.to_string()
        }
    );
    // any validated address may be allowlisted; whether it honours forwarded actions is
    // the collection's own decision, so a wrong entry is harmless (calls fail at the target)
    let unknown = deps.api.addr_make("unknown_contract");
    exec(deps.as_mut(), &a.admin, &[], add(&unknown)).unwrap();
    exec(deps.as_mut(), &a.admin, &[], remove(&unknown)).unwrap();
    let me = mock_env().contract.address;
    assert_eq!(
        exec(deps.as_mut(), &a.admin, &[], add(&me)).unwrap_err(),
        ContractError::SelfTarget {}
    );

    let r = exec(deps.as_mut(), &a.admin, &[], add(&a.good2)).unwrap();
    assert!(r.events.iter().any(|e| e.ty == "collection_added"));
    assert_eq!(collections(&deps, None, None).len(), 2);
    // pagination
    let page = collections(&deps, None, Some(1));
    assert_eq!(page.len(), 1);
    let rest = collections(&deps, Some(&page[0]), None);
    assert_eq!(rest.len(), 1);
    assert_ne!(rest[0], page[0]);

    // removal is a plain delete: the collection simply stops being a sponsored target
    let r = exec(deps.as_mut(), &a.admin, &[], remove(&a.good2)).unwrap();
    assert!(r.events.iter().any(|e| e.ty == "collection_removed"));
    assert_eq!(
        exec(deps.as_mut(), &a.admin, &[], remove(&a.good2)).unwrap_err(),
        ContractError::CollectionNotAllowed {
            collection: a.good2.to_string()
        }
    );
    assert_eq!(collections(&deps, None, None), vec![a.good.clone()]);
    // nothing is relayed there any more, revocation included: users revoke directly
    for msg in [
        burn(&a.good2, "t1"),
        approve(&a.good2, &a.marketplace, None),
        ExecuteMsg::SponsoredApproval {
            collection: a.good2.to_string(),
            action: ApprovalAction::RevokeAll {
                operator: a.marketplace.to_string(),
            },
        },
    ] {
        assert!(matches!(
            exec(deps.as_mut(), &a.alice, &[], msg).unwrap_err(),
            ContractError::CollectionNotAllowed { .. }
        ));
    }
    // re-adding restores it
    exec(deps.as_mut(), &a.admin, &[], add(&a.good2)).unwrap();
    exec(deps.as_mut(), &a.alice, &[], burn(&a.good2, "t1")).unwrap();
    exec(deps.as_mut(), &a.admin, &[], remove(&a.good2)).unwrap();

    // cap
    for i in 0..(MAX_COLLECTIONS - 1) {
        let c = deps.api.addr_make(&format!("filler{i}"));
        COLLECTIONS
            .save(deps.as_mut().storage, &c, &cosmwasm_std::Empty {})
            .unwrap();
    }
    assert_eq!(
        exec(deps.as_mut(), &a.admin, &[], add(&a.good2)).unwrap_err(),
        ContractError::TooManyCollections {
            max: MAX_COLLECTIONS
        }
    );
}

#[test]
fn operators_are_removal_only_and_admin_is_transferable() {
    let (mut deps, a) = setup(None);
    // no add path exists, even in the wire format
    assert!(
        from_json::<ExecuteMsg>(
            format!(
                r#"{{"add_allowed_operator":{{"operator":"{}"}}}}"#,
                a.other_operator
            )
            .as_bytes()
        )
        .is_err()
    );
    assert!(
        from_json::<ExecuteMsg>(
            format!(
                r#"{{"set_allowed_operators":{{"operators":["{}"]}}}}"#,
                a.other_operator
            )
            .as_bytes()
        )
        .is_err()
    );

    let rm = ExecuteMsg::RemoveAllowedOperator {
        operator: a.marketplace.to_string(),
    };
    assert_eq!(
        exec(deps.as_mut(), &a.bob, &[], rm.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );
    exec(deps.as_mut(), &a.admin, &[], rm.clone()).unwrap();
    assert_eq!(
        exec(deps.as_mut(), &a.admin, &[], rm).unwrap_err(),
        ContractError::OperatorNotFound {
            operator: a.marketplace.to_string()
        }
    );
    assert!(config(&deps).allowed_operators.is_empty());
    assert_eq!(config(&deps).former_operators, vec![a.marketplace.clone()]);
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.alice,
            &[],
            approve(&a.good, &a.marketplace, None)
        )
        .unwrap_err(),
        ContractError::OperatorNotAllowed {
            operator: a.marketplace.to_string()
        }
    );
    // but revoking the removed operator still goes through: that is the incident path
    exec(
        deps.as_mut(),
        &a.alice,
        &[],
        ExecuteMsg::SponsoredApproval {
            collection: a.good.to_string(),
            action: ApprovalAction::RevokeAll {
                operator: a.marketplace.to_string(),
            },
        },
    )
    .unwrap();
    // burns are unaffected by operator policy
    exec(deps.as_mut(), &a.alice, &[], burn(&a.good, "t1")).unwrap();

    // admin transfer; never to the proxy itself
    let me = mock_env().contract.address;
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.admin,
            &[],
            ExecuteMsg::UpdateAdmin {
                admin: me.to_string()
            }
        )
        .unwrap_err(),
        ContractError::SelfTarget {}
    );
    let handover = ExecuteMsg::UpdateAdmin {
        admin: a.bob.to_string(),
    };
    assert_eq!(
        exec(deps.as_mut(), &a.bob, &[], handover.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );
    exec(deps.as_mut(), &a.admin, &[], handover).unwrap();
    assert_eq!(config(&deps).admin, a.bob);
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.admin,
            &[],
            ExecuteMsg::RemoveCollection {
                collection: a.good.to_string()
            }
        )
        .unwrap_err(),
        ContractError::Unauthorized {}
    );
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        ExecuteMsg::RemoveCollection {
            collection: a.good.to_string(),
        },
    )
    .unwrap();
}

// ---------------------------------------------------------------------------------------
// wire format
// ---------------------------------------------------------------------------------------

#[test]
fn burn_cannot_be_smuggled_under_the_approval_key() {
    // authz budgets are per top-level key, so the approval key must not accept a burn
    assert!(
        from_json::<ExecuteMsg>(
            r#"{"sponsored_approval":{"collection":"c","action":{"burn":{"token_id":"t"}}}}"#
        )
        .is_err()
    );
    // and neither entry accepts a sender field
    assert!(
        from_json::<ExecuteMsg>(
            r#"{"sponsored_burn":{"collection":"c","token_id":"t","sender":"x"}}"#
        )
        .is_err()
    );
    assert!(
        from_json::<ExecuteMsg>(
            r#"{"sponsored_approval":{"collection":"c","sender":"x","action":{"revoke_all":{"operator":"o"}}}}"#
        )
        .is_err()
    );
    let ok: ExecuteMsg =
        from_json(r#"{"sponsored_burn":{"collection":"c","token_id":"t"}}"#).unwrap();
    assert!(matches!(ok, ExecuteMsg::SponsoredBurn { .. }));
}

#[test]
fn former_operator_revocation_survives_an_admin_handover() {
    // the incident lever must keep working after the admin address changes: both operator
    // sets are config state, not admin state, so a handover must not disturb them
    let (mut deps, a) = setup(None);
    exec(
        deps.as_mut(),
        &a.admin,
        &[],
        ExecuteMsg::RemoveAllowedOperator {
            operator: a.marketplace.to_string(),
        },
    )
    .unwrap();
    exec(
        deps.as_mut(),
        &a.admin,
        &[],
        ExecuteMsg::UpdateAdmin {
            admin: a.bob.to_string(),
        },
    )
    .unwrap();

    let cfg = config(&deps);
    assert_eq!(cfg.admin, a.bob);
    assert!(cfg.allowed_operators.is_empty());
    assert_eq!(cfg.former_operators, vec![a.marketplace.clone()]);

    // a user can still revoke the removed operator through the sponsored path
    let revoke = ExecuteMsg::SponsoredApproval {
        collection: a.good.to_string(),
        action: ApprovalAction::RevokeAll {
            operator: a.marketplace.to_string(),
        },
    };
    let r = exec(deps.as_mut(), &a.alice, &[], revoke).unwrap();
    let (_, envelope, _) = forwarded(&r);
    assert_eq!(
        envelope,
        ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
            sender: a.alice.to_string(),
            action: ProxyAction::RevokeAll {
                operator: a.marketplace.to_string()
            },
        })
    );
    // approving it is still refused, and an operator never configured here still is too
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.alice,
            &[],
            approve(&a.good, &a.marketplace, None)
        )
        .unwrap_err(),
        ContractError::OperatorNotAllowed {
            operator: a.marketplace.to_string()
        }
    );
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.alice,
            &[],
            ExecuteMsg::SponsoredApproval {
                collection: a.good.to_string(),
                action: ApprovalAction::RevokeAll {
                    operator: a.other_operator.to_string()
                },
            }
        )
        .unwrap_err(),
        ContractError::OperatorNotAllowed {
            operator: a.other_operator.to_string()
        }
    );
    // the old admin has no authority left
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.admin,
            &[],
            ExecuteMsg::RemoveCollection {
                collection: a.good.to_string()
            }
        )
        .unwrap_err(),
        ContractError::Unauthorized {}
    );
}

#[test]
fn removing_a_collection_frees_exactly_one_slot() {
    let (mut deps, a) = setup(None);
    // fill to the cap: the seeded `good` plus fillers
    for i in 0..(MAX_COLLECTIONS - 1) {
        let c = deps.api.addr_make(&format!("filler{i}"));
        COLLECTIONS
            .save(deps.as_mut().storage, &c, &cosmwasm_std::Empty {})
            .unwrap();
    }
    assert_eq!(count_collections(deps.as_ref().storage), MAX_COLLECTIONS);
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.admin,
            &[],
            ExecuteMsg::AddCollection {
                collection: a.good2.to_string()
            }
        )
        .unwrap_err(),
        ContractError::TooManyCollections {
            max: MAX_COLLECTIONS
        }
    );
    exec(
        deps.as_mut(),
        &a.admin,
        &[],
        ExecuteMsg::RemoveCollection {
            collection: a.good.to_string(),
        },
    )
    .unwrap();
    assert_eq!(
        count_collections(deps.as_ref().storage),
        MAX_COLLECTIONS - 1
    );
    exec(
        deps.as_mut(),
        &a.admin,
        &[],
        ExecuteMsg::AddCollection {
            collection: a.good2.to_string(),
        },
    )
    .unwrap();
    assert_eq!(count_collections(deps.as_ref().storage), MAX_COLLECTIONS);
}

#[test]
fn revoke_all_forwards_with_its_own_action_label() {
    let (mut deps, a) = setup(Some(60));
    let r = exec(
        deps.as_mut(),
        &a.alice,
        &[],
        ExecuteMsg::SponsoredApproval {
            collection: a.good.to_string(),
            action: ApprovalAction::RevokeAll {
                operator: a.marketplace.to_string(),
            },
        },
    )
    .unwrap();
    let (_, envelope, _) = forwarded(&r);
    assert_eq!(
        envelope,
        ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
            sender: a.alice.to_string(),
            action: ProxyAction::RevokeAll {
                operator: a.marketplace.to_string()
            },
        })
    );
    assert!(
        r.attributes
            .iter()
            .any(|x| x.key == "proxy_action" && x.value == "revoke_all")
    );
    assert!(
        r.attributes
            .iter()
            .any(|x| x.key == "action" && x.value == "sponsored_approval")
    );
}

#[test]
fn extreme_timestamps_never_panic() {
    let (mut deps, a) = setup(Some(crate::state::MAX_APPROVAL_CAP_SECONDS));
    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(u64::MAX - 5);
    for t in [
        Timestamp::from_nanos(u64::MAX),
        Timestamp::from_nanos(u64::MAX - 5),
        Timestamp::from_nanos(0),
    ] {
        let res = execute(
            deps.as_mut(),
            env.clone(),
            message_info(&a.alice, &[]),
            approve(&a.good, &a.marketplace, Some(Expiration::AtTime(t))),
        );
        // only the strictly-later, in-window timestamp passes; nothing panics
        assert_eq!(res.is_ok(), t == Timestamp::from_nanos(u64::MAX), "t={t}");
    }
}

#[test]
fn expiry_check_uses_block_time() {
    let (mut deps, a) = setup(Some(60));
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(1_000_000);
    let within = approve(
        &a.good,
        &a.marketplace,
        Some(Expiration::AtTime(Timestamp::from_seconds(1_000_060))),
    );
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&a.alice, &[]),
        within,
    )
    .unwrap();
    let beyond = approve(
        &a.good,
        &a.marketplace,
        Some(Expiration::AtTime(Timestamp::from_seconds(1_000_061))),
    );
    assert_eq!(
        execute(deps.as_mut(), env, message_info(&a.alice, &[]), beyond).unwrap_err(),
        ContractError::ApprovalExpiryOutOfBounds { max_seconds: 60 }
    );
}

use cosmwasm_std::{
    Addr, Coin, DepsMut, OwnedDeps, Response, coin, from_json,
    testing::{MockApi, MockQuerier, MockStorage, message_info, mock_dependencies, mock_env},
};
use cw721::{
    Action, Expiration,
    msg::{Cw721MigrateMsg, OperatorResponse, OwnerOfResponse},
};
use schemars::schema::{RootSchema, Schema, SchemaObject};

use crate::{
    CONTRACT_NAME, CONTRACT_VERSION,
    contract::{execute, instantiate, migrate, query},
    error::ContractError,
    msg::{
        BaseExecuteMsg, BaseQueryMsg, InstantiateMsg, ProxyAction, ProxyMsg, ProxyQueryMsg,
        ProxyableExecuteMsg, ProxyableQueryMsg,
    },
    state::TRUSTED_PROXIES,
};

type Deps = OwnedDeps<MockStorage, MockApi, MockQuerier>;

struct Actors {
    creator: Addr,
    minter: Addr,
    alice: Addr,
    bob: Addr,
    proxy: Addr,
    marketplace: Addr,
}

fn setup() -> (Deps, Actors) {
    let mut deps = mock_dependencies();
    let a = Actors {
        creator: deps.api.addr_make("creator"),
        minter: deps.api.addr_make("minter"),
        alice: deps.api.addr_make("alice"),
        bob: deps.api.addr_make("bob"),
        proxy: deps.api.addr_make("proxy"),
        marketplace: deps.api.addr_make("marketplace"),
    };
    let msg = InstantiateMsg {
        name: "Variant".to_string(),
        symbol: "VAR".to_string(),
        collection_info_extension: None,
        minter: Some(a.minter.to_string()),
        creator: Some(a.creator.to_string()),
        withdraw_address: None,
    };
    instantiate(
        deps.as_mut(),
        mock_env(),
        message_info(&a.creator, &[]),
        msg,
    )
    .unwrap();
    (deps, a)
}

fn exec(
    deps: DepsMut,
    sender: &Addr,
    funds: &[Coin],
    msg: ProxyableExecuteMsg,
) -> Result<Response, ContractError> {
    execute(deps, mock_env(), message_info(sender, funds), msg)
}

fn base(msg: BaseExecuteMsg) -> ProxyableExecuteMsg {
    ProxyableExecuteMsg::Base(msg)
}

fn proxied(sender: &Addr, action: ProxyAction) -> ProxyableExecuteMsg {
    ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute {
        sender: sender.to_string(),
        action,
    })
}

fn mint(deps: DepsMut, minter: &Addr, owner: &Addr, token_id: &str) {
    exec(
        deps,
        minter,
        &[],
        base(BaseExecuteMsg::Mint {
            token_id: token_id.to_string(),
            owner: owner.to_string(),
            token_uri: None,
            extension: None,
        }),
    )
    .unwrap();
}

fn add_proxy(deps: DepsMut, creator: &Addr, proxy: &Addr) {
    exec(
        deps,
        creator,
        &[],
        ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            proxy: proxy.to_string(),
            require_immutable: false,
        }),
    )
    .unwrap();
}

fn owner_of(deps: &Deps, token_id: &str) -> Option<String> {
    let q = ProxyableQueryMsg::Base(BaseQueryMsg::OwnerOf {
        token_id: token_id.to_string(),
        include_expired: None,
    });
    query(deps.as_ref(), mock_env(), q)
        .ok()
        .map(|b| from_json::<OwnerOfResponse>(&b).unwrap().owner)
}

fn is_operator(deps: &Deps, owner: &Addr, operator: &Addr) -> bool {
    let q = ProxyableQueryMsg::Base(BaseQueryMsg::Operator {
        owner: owner.to_string(),
        operator: operator.to_string(),
        include_expired: None,
    });
    query(deps.as_ref(), mock_env(), q)
        .map(|b| from_json::<OperatorResponse>(&b).is_ok())
        .unwrap_or(false)
}

fn trusted_proxies(deps: &Deps) -> Vec<Addr> {
    let b = query(
        deps.as_ref(),
        mock_env(),
        ProxyableQueryMsg::Proxy(ProxyQueryMsg::GetTrustedProxies {}),
    )
    .unwrap();
    from_json(&b).unwrap()
}

fn attr<'a>(r: &'a Response, key: &str) -> Option<&'a str> {
    r.attributes
        .iter()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
}

// ---------------------------------------------------------------------------------------
// identity
// ---------------------------------------------------------------------------------------

#[test]
fn instantiate_records_variant_cw2_identity() {
    let (deps, _) = setup();
    let v = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(v.contract, CONTRACT_NAME);
    assert_eq!(v.version, CONTRACT_VERSION);
    assert_ne!(v.contract, asset::CONTRACT_NAME);
    assert!(trusted_proxies(&deps).is_empty());
}

// ---------------------------------------------------------------------------------------
// proxied actions
// ---------------------------------------------------------------------------------------

#[test]
fn proxied_actions_apply_to_the_effective_sender() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);

    // approve_all as alice
    let r = exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        proxied(
            &a.alice,
            ProxyAction::ApproveAll {
                operator: a.marketplace.to_string(),
                expires: Some(Expiration::AtHeight(mock_env().block.height + 100)),
            },
        ),
    )
    .unwrap();
    assert!(is_operator(&deps, &a.alice, &a.marketplace));
    assert!(
        !is_operator(&deps, &a.proxy, &a.marketplace),
        "operator must not be keyed on the proxy"
    );
    assert_eq!(attr(&r, "proxied_by"), Some(a.proxy.as_str()));
    assert_eq!(attr(&r, "effective_sender"), Some(a.alice.as_str()));
    assert_eq!(attr(&r, "proxy_action"), Some("approve_all"));

    // revoke_all as alice
    exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        proxied(
            &a.alice,
            ProxyAction::RevokeAll {
                operator: a.marketplace.to_string(),
            },
        ),
    )
    .unwrap();
    assert!(!is_operator(&deps, &a.alice, &a.marketplace));

    // burn as alice
    exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        proxied(
            &a.alice,
            ProxyAction::Burn {
                token_id: "t1".to_string(),
            },
        ),
    )
    .unwrap();
    assert_eq!(owner_of(&deps, "t1"), None);
}

#[test]
fn untrusted_sender_cannot_use_the_envelope() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    // bob is not registered, even though alice is the real owner
    let err = exec(
        deps.as_mut(),
        &a.bob,
        &[],
        proxied(
            &a.alice,
            ProxyAction::Burn {
                token_id: "t1".to_string(),
            },
        ),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});
    assert_eq!(owner_of(&deps, "t1"), Some(a.alice.to_string()));
}

#[test]
fn trusted_proxy_sending_a_plain_message_acts_as_itself() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    // a plain Burn from the proxy is just a burn by a non-owner: rejected by the base
    let err = exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        base(BaseExecuteMsg::Burn {
            token_id: "t1".to_string(),
        }),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Asset(_)), "{err:?}");
    assert_eq!(owner_of(&deps, "t1"), Some(a.alice.to_string()));
}

#[test]
fn envelope_never_accepts_funds() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let err = exec(
        deps.as_mut(),
        &a.proxy,
        &[coin(1, "uxion")],
        proxied(
            &a.alice,
            ProxyAction::RevokeAll {
                operator: a.marketplace.to_string(),
            },
        ),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Payment(_)), "{err:?}");
}

#[test]
fn proxied_burn_is_owner_only_while_direct_burn_keeps_operator_authority() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    mint(deps.as_mut(), &a.minter, &a.alice, "t2");
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    // alice makes bob an operator, directly
    exec(
        deps.as_mut(),
        &a.alice,
        &[],
        base(BaseExecuteMsg::ApproveAll {
            operator: a.bob.to_string(),
            expires: None,
        }),
    )
    .unwrap();

    // bob through the proxy: rejected, he is not the owner
    let err = exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        proxied(
            &a.bob,
            ProxyAction::Burn {
                token_id: "t1".to_string(),
            },
        ),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::NotTokenOwner {
            token_id: "t1".to_string()
        }
    );
    assert_eq!(owner_of(&deps, "t1"), Some(a.alice.to_string()));

    // bob directly: still allowed by cw721 operator semantics (no regression)
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::Burn {
            token_id: "t2".to_string(),
        }),
    )
    .unwrap();
    assert_eq!(owner_of(&deps, "t2"), None);

    // a proxied burn of a token that does not exist surfaces the storage error, not a panic
    let err = exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        proxied(
            &a.alice,
            ProxyAction::Burn {
                token_id: "nope".to_string(),
            },
        ),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::Std(_)), "{err:?}");
}

#[test]
fn proxied_burn_respects_the_listing_guard() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    exec(
        deps.as_mut(),
        &a.alice,
        &[],
        base(BaseExecuteMsg::UpdateExtension {
            msg: asset::msg::AssetExtensionExecuteMsg::List {
                token_id: "t1".to_string(),
                price: coin(100, "uxion"),
                reservation: None,
            },
        }),
    )
    .unwrap();

    let err = exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        proxied(
            &a.alice,
            ProxyAction::Burn {
                token_id: "t1".to_string(),
            },
        ),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("cannot burn a token while it is listed"),
        "{err}"
    );
    assert_eq!(owner_of(&deps, "t1"), Some(a.alice.to_string()));
}

// ---------------------------------------------------------------------------------------
// trust management
// ---------------------------------------------------------------------------------------

#[test]
fn trust_management_is_creator_only_capped_and_self_excluding() {
    let (mut deps, a) = setup();
    let add = |p: &Addr| {
        ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            proxy: p.to_string(),
            require_immutable: false,
        })
    };
    let remove = |p: &Addr| {
        ProxyableExecuteMsg::Proxy(ProxyMsg::RemoveTrustedProxy {
            proxy: p.to_string(),
        })
    };

    // not the creator (the minter is a different role)
    assert_eq!(
        exec(deps.as_mut(), &a.minter, &[], add(&a.proxy)).unwrap_err(),
        ContractError::Unauthorized {}
    );
    assert_eq!(
        exec(deps.as_mut(), &a.bob, &[], add(&a.proxy)).unwrap_err(),
        ContractError::Unauthorized {}
    );
    // funds rejected
    assert!(matches!(
        exec(
            deps.as_mut(),
            &a.creator,
            &[coin(1, "uxion")],
            add(&a.proxy)
        )
        .unwrap_err(),
        ContractError::Payment(_)
    ));
    // self excluded
    let me = mock_env().contract.address;
    assert!(matches!(
        exec(deps.as_mut(), &a.creator, &[], add(&me)).unwrap_err(),
        ContractError::InvalidTrustedProxy { .. }
    ));

    // cap
    let extra: Vec<Addr> = (0..4)
        .map(|i| deps.api.addr_make(&format!("p{i}")))
        .collect();
    for p in &extra {
        exec(deps.as_mut(), &a.creator, &[], add(p)).unwrap();
    }
    assert_eq!(trusted_proxies(&deps).len(), 4);
    // re-adding an existing one is rejected, symmetric with removing an unknown one
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], add(&extra[0])).unwrap_err(),
        ContractError::TrustedProxyAlreadyExists {
            proxy: extra[0].to_string()
        }
    );
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], add(&a.proxy)).unwrap_err(),
        ContractError::TooManyTrustedProxies { max: 4 }
    );

    // remove
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], remove(&a.proxy)).unwrap_err(),
        ContractError::TrustedProxyNotFound {
            proxy: a.proxy.to_string()
        }
    );
    let r = exec(deps.as_mut(), &a.creator, &[], remove(&extra[0])).unwrap();
    let ev = r
        .events
        .iter()
        .find(|e| e.ty == "trusted_proxy_removed")
        .unwrap();
    assert!(ev.attributes.iter().any(|x| x.key == "collection"));
    assert!(
        ev.attributes
            .iter()
            .any(|x| x.key == "proxy" && x.value == extra[0].as_str())
    );
    assert_eq!(trusted_proxies(&deps).len(), 3);
    assert!(!trusted_proxies(&deps).contains(&extra[0]));
    // removed proxy can no longer forward
    assert_eq!(
        exec(
            deps.as_mut(),
            &extra[0],
            &[],
            proxied(
                &a.alice,
                ProxyAction::RevokeAll {
                    operator: a.marketplace.to_string()
                }
            )
        )
        .unwrap_err(),
        ContractError::Unauthorized {}
    );
}

#[test]
fn creator_cannot_renounce_while_proxies_are_registered() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let renounce = base(BaseExecuteMsg::UpdateCreatorOwnership(
        Action::RenounceOwnership,
    ));
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], renounce.clone()).unwrap_err(),
        ContractError::TrustedProxiesNotEmpty {}
    );
    // the deprecated alias targets minter ownership, so it neither orphans trust nor is it
    // affected by the guard; the creator role stays in place
    #[allow(deprecated)]
    let renounce_minter = base(BaseExecuteMsg::UpdateOwnership(Action::RenounceOwnership));
    exec(deps.as_mut(), &a.minter, &[], renounce_minter).unwrap();
    let creator = cw721::state::CREATOR
        .item
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(creator.owner, Some(a.creator.clone()));
    // a pending creator transfer does not lift the guard either
    exec(
        deps.as_mut(),
        &a.creator,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::TransferOwnership {
                new_owner: a.bob.to_string(),
                expiry: None,
            },
        )),
    )
    .unwrap();
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], renounce.clone()).unwrap_err(),
        ContractError::TrustedProxiesNotEmpty {}
    );
    // removal is creator-only and non-payable
    let remove = ProxyableExecuteMsg::Proxy(ProxyMsg::RemoveTrustedProxy {
        proxy: a.proxy.to_string(),
    });
    assert_eq!(
        exec(deps.as_mut(), &a.bob, &[], remove.clone()).unwrap_err(),
        ContractError::Unauthorized {}
    );
    assert!(matches!(
        exec(deps.as_mut(), &a.creator, &[coin(1, "uxion")], remove).unwrap_err(),
        ContractError::Payment(_)
    ));
    // completing the transfer is allowed and clears every registered proxy, so the new
    // creator starts from a clean slate and may renounce right away
    let r = exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("1"));
    assert!(trusted_proxies(&deps).is_empty());
    exec(deps.as_mut(), &a.bob, &[], renounce).unwrap();
}

#[test]
fn creator_handover_cannot_leave_or_sneak_in_a_trusted_proxy() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let sneaky = deps.api.addr_make("sneaky");

    // creator proposes a handover to bob
    exec(
        deps.as_mut(),
        &a.creator,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::TransferOwnership {
                new_owner: a.bob.to_string(),
                expiry: None,
            },
        )),
    )
    .unwrap();

    // while pending: adding is refused, removing is still allowed, existing proxies keep working
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.creator,
            &[],
            ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
                proxy: sneaky.to_string(),
                require_immutable: false,
            }),
        )
        .unwrap_err(),
        ContractError::CreatorTransferPending {}
    );
    exec(
        deps.as_mut(),
        &a.proxy,
        &[],
        proxied(
            &a.alice,
            ProxyAction::ApproveAll {
                operator: a.marketplace.to_string(),
                expires: None,
            },
        ),
    )
    .unwrap();
    assert!(is_operator(&deps, &a.alice, &a.marketplace));
    let second = deps.api.addr_make("second");
    // seed a second entry directly to prove clearing handles more than one
    TRUSTED_PROXIES
        .save(deps.as_mut().storage, &second, &cosmwasm_std::Empty {})
        .unwrap();
    assert_eq!(trusted_proxies(&deps).len(), 2);

    // bob accepts: everything the old creator registered is gone
    let r = exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("2"));
    assert!(trusted_proxies(&deps).is_empty());
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.proxy,
            &[],
            proxied(
                &a.alice,
                ProxyAction::Burn {
                    token_id: "t1".to_string()
                }
            ),
        )
        .unwrap_err(),
        ContractError::Unauthorized {}
    );
    assert_eq!(owner_of(&deps, "t1"), Some(a.alice.to_string()));

    // the old creator has no say any more; bob re-registers what he trusts
    assert_eq!(
        exec(
            deps.as_mut(),
            &a.creator,
            &[],
            ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
                proxy: a.proxy.to_string(),
                require_immutable: false,
            }),
        )
        .unwrap_err(),
        ContractError::Unauthorized {}
    );
    add_proxy(deps.as_mut(), &a.bob, &a.proxy);
    assert_eq!(trusted_proxies(&deps), vec![a.proxy.clone()]);

    // an accepted transfer with nothing registered is a no-op clear
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::TransferOwnership {
                new_owner: a.creator.to_string(),
                expiry: None,
            },
        )),
    )
    .unwrap();
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        ProxyableExecuteMsg::Proxy(ProxyMsg::RemoveTrustedProxy {
            proxy: a.proxy.to_string(),
        }),
    )
    .unwrap();
    let r = exec(
        deps.as_mut(),
        &a.creator,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("0"));
}

fn propose(deps: DepsMut, from: &Addr, to: &Addr, expiry: Option<Expiration>) {
    exec(
        deps,
        from,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::TransferOwnership {
                new_owner: to.to_string(),
                expiry,
            },
        )),
    )
    .unwrap();
}

fn add_proxy_msg(p: &Addr) -> ProxyableExecuteMsg {
    ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
        proxy: p.to_string(),
        require_immutable: false,
    })
}

#[test]
fn expired_transfer_does_not_block_adds_and_keeps_proxies() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let extra = deps.api.addr_make("extra");
    let height = mock_env().block.height;

    propose(
        deps.as_mut(),
        &a.creator,
        &a.bob,
        Some(Expiration::AtHeight(height + 10)),
    );
    // live: blocked
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], add_proxy_msg(&extra)).unwrap_err(),
        ContractError::CreatorTransferPending {}
    );

    // past the deadline: the proposal is dead, adds work again, nothing was cleared
    let mut env = mock_env();
    env.block.height = height + 10;
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&a.creator, &[]),
        add_proxy_msg(&extra),
    )
    .unwrap();
    assert_eq!(trusted_proxies(&deps).len(), 2);
    // and bob can no longer accept, so the proxies stay
    execute(
        deps.as_mut(),
        env,
        message_info(&a.bob, &[]),
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap_err();
    assert_eq!(trusted_proxies(&deps).len(), 2);
    let owner = cw721::state::CREATOR
        .item
        .load(deps.as_ref().storage)
        .unwrap()
        .owner;
    assert_eq!(owner, Some(a.creator.clone()));
}

#[test]
fn cancelling_a_transfer_keeps_proxies_and_reenables_adds() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let extra = deps.api.addr_make("extra");

    propose(deps.as_mut(), &a.creator, &a.bob, None);
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], add_proxy_msg(&extra)).unwrap_err(),
        ContractError::CreatorTransferPending {}
    );

    // cw-ownable has no cancel action: proposing to yourself is the cancel idiom.
    // bob is no longer the pending owner, so adds are allowed immediately ...
    propose(deps.as_mut(), &a.creator, &a.creator, None);
    exec(deps.as_mut(), &a.creator, &[], add_proxy_msg(&extra)).unwrap();
    assert_eq!(trusted_proxies(&deps).len(), 2);
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap_err();
    // ... and self-accepting to tidy up changes no control, so nothing is cleared
    let r = exec(
        deps.as_mut(),
        &a.creator,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("0"));
    assert_eq!(trusted_proxies(&deps).len(), 2);
}

#[test]
fn replaced_transfer_stays_blocked_until_the_final_acceptor_takes_over() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let carol = deps.api.addr_make("carol");
    let extra = deps.api.addr_make("extra");

    propose(deps.as_mut(), &a.creator, &a.bob, None);
    // overwrite with a proposal to carol: still live, still blocked
    propose(deps.as_mut(), &a.creator, &carol, None);
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], add_proxy_msg(&extra)).unwrap_err(),
        ContractError::CreatorTransferPending {}
    );
    // bob was superseded
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap_err();
    // the pending owner is not the creator yet and cannot add either
    assert_eq!(
        exec(deps.as_mut(), &carol, &[], add_proxy_msg(&extra)).unwrap_err(),
        ContractError::Unauthorized {}
    );
    // carol accepts: control changes, proxies cleared
    let r = exec(
        deps.as_mut(),
        &carol,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("1"));
    assert!(trusted_proxies(&deps).is_empty());
}

#[test]
fn renounce_with_pending_transfer_follows_the_proxy_guard() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    propose(deps.as_mut(), &a.creator, &a.bob, None);
    let renounce = base(BaseExecuteMsg::UpdateCreatorOwnership(
        Action::RenounceOwnership,
    ));
    // proxies registered: refused regardless of the pending transfer
    assert_eq!(
        exec(deps.as_mut(), &a.creator, &[], renounce.clone()).unwrap_err(),
        ContractError::TrustedProxiesNotEmpty {}
    );
    // removal is allowed during the window; then renouncing cancels the transfer too
    exec(
        deps.as_mut(),
        &a.creator,
        &[],
        ProxyableExecuteMsg::Proxy(ProxyMsg::RemoveTrustedProxy {
            proxy: a.proxy.to_string(),
        }),
    )
    .unwrap();
    exec(deps.as_mut(), &a.creator, &[], renounce).unwrap();
    let ownership = cw721::state::CREATOR
        .item
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(ownership.owner, None);
    assert_eq!(ownership.pending_owner, None);
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap_err();
}

#[test]
fn failed_acceptance_changes_nothing() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    // nothing pending: acceptance fails at the base and no clearing happens
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap_err();
    assert_eq!(trusted_proxies(&deps), vec![a.proxy.clone()]);
    // pending to bob, but a stranger tries to accept
    propose(deps.as_mut(), &a.creator, &a.bob, None);
    let stranger = deps.api.addr_make("stranger");
    exec(
        deps.as_mut(),
        &stranger,
        &[],
        base(BaseExecuteMsg::UpdateCreatorOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap_err();
    assert_eq!(trusted_proxies(&deps), vec![a.proxy.clone()]);
    let owner = cw721::state::CREATOR
        .item
        .load(deps.as_ref().storage)
        .unwrap()
        .owner;
    assert_eq!(owner, Some(a.creator.clone()));
}

#[test]
fn minter_ownership_changes_do_not_touch_proxy_trust() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let extra = deps.api.addr_make("extra");
    // a pending *minter* transfer is unrelated to creator trust
    exec(
        deps.as_mut(),
        &a.minter,
        &[],
        base(BaseExecuteMsg::UpdateMinterOwnership(
            Action::TransferOwnership {
                new_owner: a.bob.to_string(),
                expiry: None,
            },
        )),
    )
    .unwrap();
    exec(deps.as_mut(), &a.creator, &[], add_proxy_msg(&extra)).unwrap();
    exec(
        deps.as_mut(),
        &a.bob,
        &[],
        base(BaseExecuteMsg::UpdateMinterOwnership(
            Action::AcceptOwnership,
        )),
    )
    .unwrap();
    assert_eq!(trusted_proxies(&deps).len(), 2);
}

// ---------------------------------------------------------------------------------------
// pass-through
// ---------------------------------------------------------------------------------------

#[test]
fn base_messages_behave_identically_to_the_base_contract() {
    // Differential: the same sequence through the wrapper and through asset_base directly
    // must yield identical responses and identical owner state.
    let (mut wrapped, a) = setup();
    let mut direct = mock_dependencies();
    {
        let msg = InstantiateMsg {
            name: "Variant".to_string(),
            symbol: "VAR".to_string(),
            collection_info_extension: None,
            minter: Some(a.minter.to_string()),
            creator: Some(a.creator.to_string()),
            withdraw_address: None,
        };
        asset::contracts::asset_base::instantiate(
            direct.as_mut(),
            mock_env(),
            message_info(&a.creator, &[]),
            msg,
        )
        .unwrap();
    }

    let steps: Vec<(Addr, BaseExecuteMsg)> = vec![
        (
            a.minter.clone(),
            BaseExecuteMsg::Mint {
                token_id: "t1".to_string(),
                owner: a.alice.to_string(),
                token_uri: Some("ipfs://x".to_string()),
                extension: None,
            },
        ),
        (
            a.alice.clone(),
            BaseExecuteMsg::UpdateExtension {
                msg: asset::msg::AssetExtensionExecuteMsg::List {
                    token_id: "t1".to_string(),
                    price: coin(100, "uxion"),
                    reservation: None,
                },
            },
        ),
        (
            a.alice.clone(),
            BaseExecuteMsg::UpdateExtension {
                msg: asset::msg::AssetExtensionExecuteMsg::Delist {
                    token_id: "t1".to_string(),
                },
            },
        ),
        (
            a.alice.clone(),
            BaseExecuteMsg::TransferNft {
                recipient: a.bob.to_string(),
                token_id: "t1".to_string(),
            },
        ),
        (
            a.bob.clone(),
            BaseExecuteMsg::Burn {
                token_id: "t1".to_string(),
            },
        ),
    ];
    for (sender, msg) in steps {
        let via_wrapper = exec(wrapped.as_mut(), &sender, &[], base(msg.clone())).unwrap();
        let via_base = asset::contracts::asset_base::execute(
            direct.as_mut(),
            mock_env(),
            message_info(&sender, &[]),
            msg,
        )
        .unwrap();
        assert_eq!(via_wrapper, via_base);
    }
    assert_eq!(owner_of(&wrapped, "t1"), None);
}

// ---------------------------------------------------------------------------------------
// serde
// ---------------------------------------------------------------------------------------

#[test]
fn untagged_dispatch_is_unambiguous() {
    let p: ProxyableExecuteMsg =
        from_json(r#"{"proxy_execute":{"sender":"alice","action":{"burn":{"token_id":"t1"}}}}"#)
            .unwrap();
    assert!(matches!(
        p,
        ProxyableExecuteMsg::Proxy(ProxyMsg::ProxyExecute { .. })
    ));

    let b: ProxyableExecuteMsg = from_json(r#"{"burn":{"token_id":"t1"}}"#).unwrap();
    assert!(matches!(
        b,
        ProxyableExecuteMsg::Base(BaseExecuteMsg::Burn { .. })
    ));

    // mixed keys are not a valid externally tagged enum on either arm
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"proxy_execute":{"sender":"alice","action":{"burn":{"token_id":"t1"}}},"burn":{"token_id":"t1"}}"#
        )
        .is_err()
    );
    // unknown fields inside a proxy message are rejected
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"proxy_execute":{"sender":"alice","action":{"burn":{"token_id":"t1"}},"extra":1}}"#
        )
        .is_err()
    );
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"proxy_execute":{"sender":"alice","action":{"burn":{"token_id":"t1","x":1}}}}"#
        )
        .is_err()
    );
    // wrong case is not a proxy message and not a base message
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"ProxyExecute":{"sender":"alice","action":{"burn":{"token_id":"t1"}}}}"#
        )
        .is_err()
    );
    // reversed mixed-key order and duplicate tags are rejected too
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"burn":{"token_id":"t1"},"proxy_execute":{"sender":"alice","action":{"burn":{"token_id":"t1"}}}}"#
        )
        .is_err()
    );
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"proxy_execute":{"sender":"alice","action":{"burn":{"token_id":"t1"}}},"proxy_execute":{"sender":"bob","action":{"burn":{"token_id":"t1"}}}}"#
        )
        .is_err()
    );
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"proxy_execute":{"sender":"alice","sender":"bob","action":{"burn":{"token_id":"t1"}}}}"#
        )
        .is_err()
    );
    // there is no nesting: an envelope is not a ProxyAction
    assert!(
        from_json::<ProxyableExecuteMsg>(
            r#"{"proxy_execute":{"sender":"alice","action":{"proxy_execute":{"sender":"bob","action":{"burn":{"token_id":"t1"}}}}}}"#
        )
        .is_err()
    );
    // a base message can never be smuggled through as a proxy message
    assert!(from_json::<ProxyMsg>(r#"{"burn":{"token_id":"t1"}}"#).is_err());
    // and proxy names are not base variants
    assert!(from_json::<BaseExecuteMsg>(r#"{"proxy_execute":{}}"#).is_err());
    assert!(from_json::<BaseExecuteMsg>(r#"{"add_trusted_proxy":{"proxy":"x"}}"#).is_err());
    assert!(from_json::<BaseExecuteMsg>(r#"{"remove_trusted_proxy":{"proxy":"x"}}"#).is_err());
}

#[test]
fn query_envelope_is_unambiguous_and_fail_closed() {
    let p: ProxyableQueryMsg = from_json(r#"{"get_trusted_proxies":{}}"#).unwrap();
    assert!(matches!(
        p,
        ProxyableQueryMsg::Proxy(ProxyQueryMsg::GetTrustedProxies {})
    ));
    let b: ProxyableQueryMsg =
        from_json(r#"{"owner_of":{"token_id":"t1","include_expired":null}}"#).unwrap();
    assert!(matches!(
        b,
        ProxyableQueryMsg::Base(BaseQueryMsg::OwnerOf { .. })
    ));

    // mixed proxy and base keys, either order
    assert!(
        from_json::<ProxyableQueryMsg>(
            r#"{"get_trusted_proxies":{},"owner_of":{"token_id":"t1"}}"#
        )
        .is_err()
    );
    assert!(
        from_json::<ProxyableQueryMsg>(
            r#"{"owner_of":{"token_id":"t1"},"get_trusted_proxies":{}}"#
        )
        .is_err()
    );
    // unknown field inside the proxy query, wrong case, duplicate tag
    assert!(from_json::<ProxyableQueryMsg>(r#"{"get_trusted_proxies":{"x":1}}"#).is_err());
    assert!(from_json::<ProxyableQueryMsg>(r#"{"GetTrustedProxies":{}}"#).is_err());
    assert!(
        from_json::<ProxyableQueryMsg>(r#"{"get_trusted_proxies":{},"get_trusted_proxies":{}}"#)
            .is_err()
    );
    // a base query name is never a proxy query and vice versa
    assert!(from_json::<ProxyQueryMsg>(r#"{"owner_of":{"token_id":"t1"}}"#).is_err());
    assert!(from_json::<BaseQueryMsg>(r#"{"get_trusted_proxies":{}}"#).is_err());
    // the proxy query behaves at the entrypoint
    let (deps, _) = setup();
    let raw: ProxyableQueryMsg = from_json(r#"{"get_trusted_proxies":{}}"#).unwrap();
    let out: Vec<Addr> = from_json(query(deps.as_ref(), mock_env(), raw).unwrap()).unwrap();
    assert!(out.is_empty());
}

/// Collect the externally tagged variant names of an enum schema.
fn variant_names(root: &RootSchema) -> Vec<String> {
    fn walk(schema: &SchemaObject, out: &mut Vec<String>) {
        if let Some(obj) = &schema.object {
            out.extend(obj.required.iter().cloned());
        }
        if let Some(e) = &schema.enum_values {
            out.extend(e.iter().filter_map(|v| v.as_str().map(str::to_string)));
        }
        if let Some(sub) = &schema.subschemas {
            for list in [&sub.one_of, &sub.any_of, &sub.all_of]
                .into_iter()
                .flatten()
            {
                for s in list {
                    if let Schema::Object(o) = s {
                        walk(o, out);
                    }
                }
            }
        }
    }
    let mut out = vec![];
    walk(&root.schema, &mut out);
    out
}

#[test]
fn variant_names_are_disjoint() {
    let base_exec = variant_names(&schemars::schema_for!(BaseExecuteMsg));
    let proxy_exec = variant_names(&schemars::schema_for!(ProxyMsg));
    assert!(!base_exec.is_empty() && !proxy_exec.is_empty());
    for name in &proxy_exec {
        assert!(!base_exec.contains(name), "execute variant clash: {name}");
    }
    let base_query = variant_names(&schemars::schema_for!(BaseQueryMsg));
    let proxy_query = variant_names(&schemars::schema_for!(ProxyQueryMsg));
    for name in &proxy_query {
        assert!(!base_query.contains(name), "query variant clash: {name}");
    }
}

// ---------------------------------------------------------------------------------------
// migration
// ---------------------------------------------------------------------------------------

fn no_update() -> Cw721MigrateMsg {
    Cw721MigrateMsg::WithUpdate {
        minter: None,
        creator: None,
    }
}

#[test]
fn migrate_from_base_asset_clears_dormant_proxies() {
    let (mut deps, a) = setup();
    mint(deps.as_mut(), &a.minter, &a.alice, "t1");
    // pretend this storage came from a base `asset` 0.2.0 that once was proxyable
    cw2::set_contract_version(deps.as_mut().storage, "asset", "0.2.0").unwrap();
    TRUSTED_PROXIES
        .save(deps.as_mut().storage, &a.proxy, &cosmwasm_std::Empty {})
        .unwrap();

    let r = migrate(deps.as_mut(), mock_env(), no_update()).unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("1"));
    assert!(trusted_proxies(&deps).is_empty());
    let v = cw2::get_contract_version(deps.as_ref().storage).unwrap();
    assert_eq!(
        (v.contract.as_str(), v.version.as_str()),
        (CONTRACT_NAME, CONTRACT_VERSION)
    );
    // tokens and roles intact
    assert_eq!(owner_of(&deps, "t1"), Some(a.alice.to_string()));
    let creator = cw721::state::CREATOR
        .item
        .load(deps.as_ref().storage)
        .unwrap();
    assert_eq!(creator.owner, Some(a.creator.clone()));
}

#[test]
fn migrate_from_base_asset_clears_oversized_imported_state() {
    // more entries than the registration API can ever create; all must go
    let (mut deps, _) = setup();
    cw2::set_contract_version(deps.as_mut().storage, "asset", "0.1.0").unwrap();
    for i in 0..9 {
        let p = deps.api.addr_make(&format!("dormant{i}"));
        TRUSTED_PROXIES
            .save(deps.as_mut().storage, &p, &cosmwasm_std::Empty {})
            .unwrap();
    }
    let r = migrate(deps.as_mut(), mock_env(), no_update()).unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("9"));
    assert!(!crate::state::has_trusted_proxies(deps.as_ref().storage));
}

#[test]
fn migrate_between_variant_versions_keeps_proxies() {
    let (mut deps, a) = setup();
    add_proxy(deps.as_mut(), &a.creator, &a.proxy);
    let r = migrate(deps.as_mut(), mock_env(), no_update()).unwrap();
    assert_eq!(attr(&r, "trusted_proxies_cleared"), Some("0"));
    assert_eq!(trusted_proxies(&deps), vec![a.proxy.clone()]);
}

#[test]
fn migrate_rejects_unknown_sources() {
    let (mut deps, _) = setup();
    cw2::set_contract_version(deps.as_mut().storage, "something-else", "0.1.0").unwrap();
    assert_eq!(
        migrate(deps.as_mut(), mock_env(), no_update()).unwrap_err(),
        ContractError::InvalidMigration {
            contract: "something-else".to_string(),
            version: "0.1.0".to_string()
        }
    );
    cw2::set_contract_version(deps.as_mut().storage, "asset", "9.9.9").unwrap();
    assert!(matches!(
        migrate(deps.as_mut(), mock_env(), no_update()).unwrap_err(),
        ContractError::InvalidMigration { .. }
    ));
}

// ---------------------------------------------------------------------------------------
// optional immutability check (audit item I)
// ---------------------------------------------------------------------------------------

/// Make the mock querier describe some addresses as contracts.
fn describe_contracts(deps: &mut Deps, infos: Vec<(Addr, u64, Option<Addr>)>) {
    deps.querier.update_wasm(move |q| match q {
        cosmwasm_std::WasmQuery::ContractInfo { contract_addr } => {
            match infos.iter().find(|(a, _, _)| a.as_str() == contract_addr) {
                Some((_, code_id, admin)) => {
                    cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                        cosmwasm_std::to_json_binary(&serde_json::json!({
                            "code_id": code_id,
                            "creator": "creator",
                            "admin": admin.as_ref().map(|a| a.to_string()),
                            "pinned": false,
                            "ibc_port": null,
                        }))
                        .unwrap(),
                    ))
                }
                None => {
                    cosmwasm_std::SystemResult::Err(cosmwasm_std::SystemError::NoSuchContract {
                        addr: contract_addr.clone(),
                    })
                }
            }
        }
        _ => cosmwasm_std::SystemResult::Err(cosmwasm_std::SystemError::UnsupportedRequest {
            kind: "only contract info".to_string(),
        }),
    });
}

fn event_attr<'a>(r: &'a Response, ty: &str, key: &str) -> Option<&'a str> {
    r.events
        .iter()
        .find(|e| e.ty == ty)?
        .attributes
        .iter()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
}

#[test]
fn require_immutable_is_opt_in_and_checks_the_chain() {
    let (mut deps, a) = setup();
    let immutable = deps.api.addr_make("immutable_proxy");
    let mutable = deps.api.addr_make("mutable_proxy");
    let eoa = deps.api.addr_make("some_wallet");
    let upgrader = deps.api.addr_make("upgrader");
    describe_contracts(
        &mut deps,
        vec![
            (immutable.clone(), 42, None),
            (mutable.clone(), 43, Some(upgrader.clone())),
        ],
    );
    let add = |p: &Addr, strict: bool| {
        ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            proxy: p.to_string(),
            require_immutable: strict,
        })
    };

    // strict: only an admin-less contract passes
    let r = exec(deps.as_mut(), &a.creator, &[], add(&immutable, true)).unwrap();
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "proxy_kind"),
        Some("contract")
    );
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "proxy_code_id"),
        Some("42")
    );
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "proxy_admin"),
        Some("none")
    );
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "require_immutable"),
        Some("true")
    );
    assert!(matches!(
        exec(deps.as_mut(), &a.creator, &[], add(&mutable, true)).unwrap_err(),
        ContractError::InvalidTrustedProxy { reason } if reason.contains("wasm admin")
    ));
    assert!(matches!(
        exec(deps.as_mut(), &a.creator, &[], add(&eoa, true)).unwrap_err(),
        ContractError::InvalidTrustedProxy { reason } if reason.contains("not a contract")
    ));
    assert_eq!(trusted_proxies(&deps), vec![immutable.clone()]);

    // non-strict: everything is accepted, but the event still tells indexers what it is
    let r = exec(deps.as_mut(), &a.creator, &[], add(&mutable, false)).unwrap();
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "proxy_kind"),
        Some("contract")
    );
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "proxy_admin"),
        Some(upgrader.as_str())
    );
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "require_immutable"),
        Some("false")
    );
    let r = exec(deps.as_mut(), &a.creator, &[], add(&eoa, false)).unwrap();
    assert_eq!(
        event_attr(&r, "trusted_proxy_added", "proxy_kind"),
        Some("account")
    );
    assert_eq!(trusted_proxies(&deps).len(), 3);

    // the flag is optional on the wire and defaults to off
    let legacy: ProxyableExecuteMsg = from_json(format!(
        r#"{{"add_trusted_proxy":{{"proxy":"{}"}}}}"#,
        a.proxy
    ))
    .unwrap();
    assert!(matches!(
        legacy,
        ProxyableExecuteMsg::Proxy(ProxyMsg::AddTrustedProxy {
            require_immutable: false,
            ..
        })
    ));
    // a strict failure leaves no trace
    let strict_fail = exec(deps.as_mut(), &a.creator, &[], add(&a.bob, true));
    assert!(strict_fail.is_err());
    assert_eq!(trusted_proxies(&deps).len(), 3);
}

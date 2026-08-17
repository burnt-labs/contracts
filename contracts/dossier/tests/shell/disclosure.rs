use crate::common::*;

#[test]
fn shell_disclosure_fail_and_prune() {
    let (mut deps, env, owner, orch, v) = setup_test();
    let contract_addr = env.contract.address.to_string();
    let chain_id = env.block.chain_id.clone();
    let id = admit_id(&mut deps, &env, &owner, b"ct-1");
    accept(
        &mut deps,
        &env,
        &orch,
        &v,
        &contract_addr,
        &chain_id,
        id,
        b"ct-1",
    );

    let resp = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::CreateDisclosure {
            encrypted_request_blob: ev(b"req-blob"),
        },
    )
    .unwrap();
    let disclosure_id: u64 = resp
        .attributes
        .iter()
        .find(|a| a.key == "disclosure_id")
        .unwrap()
        .value
        .parse()
        .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    let pinned = state.pending_disclosures[&disclosure_id].entries_root_at_create;

    let pk_c_hash = sha256(b"pk_C");
    let att = v.attest(&Binding::Fail {
        chain_id: &chain_id,
        contract_address: &contract_addr,
        disclosure_id,
        reason: FailReason::PredicateUndefined,
        request_blob_hash: sha256(&ev(b"req-blob")),
        fail_reason_ciphertext_hash: sha256(b"detail"),
        snapshot_root: pinned,
        pk_slot: pk_c_hash,
        enclave_pubkey_hash: sha256(&pad48(b"pk-enclave")),
    });
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&orch, &[]),
        ExecuteMsg::FailDisclosure {
            disclosure_id,
            reason: FailReason::PredicateUndefined,
            fail_reason_ciphertext: b"detail".to_vec(),
            snapshot_root: pinned,
            pk_slot: pk_c_hash,
            attestation: att,
        },
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_disclosures.is_empty());
    assert_eq!(state.disclosures.len(), 1);
    let fd = &state.disclosures[&disclosure_id];
    assert!(matches!(
        fd.outcome,
        DisclosureOutcome::Failed {
            reason: FailReason::PredicateUndefined,
            ..
        }
    ));
    assert_eq!(fd.snapshot_root, pinned);

    let mut future_env = env.clone();
    future_env.block.time = Timestamp::from_seconds(T0 + RETENTION_WINDOW_SECONDS);
    execute(
        deps.as_mut(),
        future_env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::PruneDisclosure { disclosure_id },
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), future_env, QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.disclosures.is_empty());
}

#[test]
fn shell_cancel_disclosure() {
    let (mut deps, env, owner, orch, v) = setup_test();
    let contract_addr = env.contract.address.to_string();
    let chain_id = env.block.chain_id.clone();
    let id = admit_id(&mut deps, &env, &owner, b"ct-1");
    accept(
        &mut deps,
        &env,
        &orch,
        &v,
        &contract_addr,
        &chain_id,
        id,
        b"ct-1",
    );

    let resp = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::CreateDisclosure {
            encrypted_request_blob: ev(b"req-blob"),
        },
    )
    .unwrap();
    let disclosure_id: u64 = resp
        .attributes
        .iter()
        .find(|a| a.key == "disclosure_id")
        .unwrap()
        .value
        .parse()
        .unwrap();

    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::CancelPendingDisclosure { disclosure_id },
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_disclosures.is_empty());
}

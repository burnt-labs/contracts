use crate::common::*;

#[test]
fn full_admission_and_disclosure_flow() {
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
    assert_eq!(pinned, state.entries_root);

    let fulfil_att = v.attest(&Binding::Fulfil {
        chain_id: &chain_id,
        contract_address: &contract_addr,
        disclosure_id,
        request_blob_hash: sha256(&ev(b"req-blob")),
        disclosure_ciphertext_hash: sha256(b"result-ct"),
        snapshot_root: pinned,
        pk_c_hash: sha256(b"pk-consumer"),
        enclave_pubkey_hash: sha256(&pad48(b"pk-enclave")),
    });
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&orch, &[]),
        ExecuteMsg::FulfilDisclosure {
            disclosure_id,
            disclosure_ciphertext: b"result-ct".to_vec(),
            snapshot_root: pinned,
            pk_c_hash: sha256(b"pk-consumer"),
            attestation: fulfil_att,
        },
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env, QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_disclosures.is_empty());
    assert_eq!(state.disclosures.len(), 1);
    assert_eq!(state.entries.len(), 1);
    assert_eq!(state.next_id, 3);
}

#[test]
fn shell_admission_reject_clears_pending() {
    let (mut deps, env, owner, orch, v) = setup_test();
    let contract_addr = env.contract.address.to_string();
    let chain_id = env.block.chain_id.clone();
    let id = admit_id(&mut deps, &env, &owner, b"ct-1");

    let ct_hash = sha256(&ev(b"ct-1"));
    let att = v.attest(&Binding::AdmissionReject {
        chain_id: &chain_id,
        contract_address: &contract_addr,
        admission_id: id,
        reason: AdmissionRejectReason::SchemaMismatch,
        ciphertext_hash: ct_hash,
        schema_id: "schema-a",
        enclave_pubkey_hash: sha256(&pad48(b"pk-enclave")),
    });
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&orch, &[]),
        ExecuteMsg::AdmissionReject {
            admission_id: id,
            reason: AdmissionRejectReason::SchemaMismatch,
            attestation: att,
        },
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_admissions.is_empty());
    assert!(state.entries.is_empty());
    assert_eq!(state.next_id, 2);
}

#[test]
fn shell_cancel_admission() {
    let (mut deps, env, owner, _orch, _v) = setup_test();
    let id = admit_id(&mut deps, &env, &owner, b"ct-1");

    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::CancelPendingAdmission { admission_id: id },
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_admissions.is_empty());
}

#[test]
fn shell_revoke_recomputes_root() {
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

    let id2 = admit_id(&mut deps, &env, &owner, b"ct-2");
    accept(
        &mut deps,
        &env,
        &orch,
        &v,
        &contract_addr,
        &chain_id,
        id2,
        b"ct-2",
    );

    let state_before: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    let root_before = state_before.entries_root;

    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::Revoke { entry_id: id },
    )
    .unwrap();

    let state_after: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    assert_eq!(state_after.entries.len(), 1);
    assert!(!state_after.entries.contains_key(&id));
    assert_ne!(state_after.entries_root, root_before);
    assert_eq!(
        state_after.entries_root,
        merkle::entries_root_of_map(&state_after.entries)
    );
}

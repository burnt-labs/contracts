use crate::common::*;

#[test]
fn shell_key_rotation_full_cycle() {
    let (mut deps, env, owner, _orch, v) = setup_test();
    let contract_addr = env.contract.address.to_string();

    let path = dossier_derivation_path(&contract_addr);
    let rot_att = v.attest_key(
        &KeyBinding {
            pubkey: &pad48(b"pk-new"),
            contract_address: &contract_addr,
            derivation_path: &path,
            purpose: KeyPurpose::Rotate,
        },
        T0,
    );
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::ProposeKeyRotation {
            new_pubkey: pad48(b"pk-new"),
            attestation: rot_att,
        },
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_enclave_key.is_some());

    let mut future_env = env.clone();
    future_env.block.time = Timestamp::from_seconds(T0 + KEY_ROTATION_TIMELOCK_SECONDS);
    execute(
        deps.as_mut(),
        future_env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::FinalizeKeyRotation {},
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), future_env, QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_enclave_key.is_none());
    assert_eq!(state.enclave_pubkey.as_deref(), Some(&pad48(b"pk-new")[..]));
}

#[test]
fn shell_cancel_key_rotation() {
    let (mut deps, env, owner, _orch, v) = setup_test();
    let contract_addr = env.contract.address.to_string();
    let path = dossier_derivation_path(&contract_addr);
    let rot_att = v.attest_key(
        &KeyBinding {
            pubkey: &pad48(b"pk-new"),
            contract_address: &contract_addr,
            derivation_path: &path,
            purpose: KeyPurpose::Rotate,
        },
        T0,
    );
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::ProposeKeyRotation {
            new_pubkey: pad48(b"pk-new"),
            attestation: rot_att,
        },
    )
    .unwrap();

    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::CancelKeyRotation {},
    )
    .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    assert!(state.pending_enclave_key.is_none());
    assert_eq!(
        state.enclave_pubkey.as_deref(),
        Some(&pad48(b"pk-enclave")[..])
    );
}

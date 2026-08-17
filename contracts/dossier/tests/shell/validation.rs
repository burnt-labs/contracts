use crate::common::*;

#[test]
fn shell_rejects_non_owner_operations() {
    let (mut deps, env, _owner, _orch, _v) = setup_test();
    let stranger = deps.api.addr_make("mallory");

    // Admit is permissionless; the remaining owner-gated ops still reject.
    let err = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&stranger, &[]),
        ExecuteMsg::CreateDisclosure {
            encrypted_request_blob: ev(b"req"),
        },
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "REJECT_UNAUTHORIZED");

    let err = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&stranger, &[]),
        ExecuteMsg::RegisterEnclaveKey {
            pubkey: pad48(b"pk-other"),
            attestation: vec![0x01],
        },
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "REJECT_UNAUTHORIZED");
}

#[test]
fn shell_rejects_malformed_envelope() {
    let (mut deps, env, owner, _orch, _v) = setup_test();

    let err = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::Admit {
            schema_id: "schema-a".into(),
            ciphertext: b"raw-bytes".to_vec(),
            proof_blob: b"proof".to_vec(),
            provenance: Provenance {
                attestor: "bank-x".into(),
                proof_type: ProofType::ZkTls,
            },
        },
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "REJECT_INVALID_ENVELOPE");

    let err = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::CreateDisclosure {
            encrypted_request_blob: b"raw".to_vec(),
        },
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "REJECT_INVALID_ENVELOPE");
}

use crate::common::*;


const REAL_PUBKEY_HEX: &str =
    "99088cb69fa486be6d082bfdb3dd647c10f261bde1030dba629d5335416d6fdc30f5818cf8dcfe849bd29507b661382c";

/// Hybrid BLS12-381-TLE + AES-256-GCM envelope for admission payload data.
const REAL_ENVELOPE_JSON: &str =
    r#"{"version":1,"keyAlgorithm":"BLS12-381-TLE","payloadAlgorithm":"AES-256-GCM","tleKeyCiphertextHex":"98f5df8a86da61df7e12343c32f1b9a77c86399760f7c7bc41d48601a25074601252b4db260c0f963860d6f00d5b21bbe69e9f510cf047ac957751b60a5cae69cdd5a8cd7dd4889525cdec9a0886651b584daa3aa14e3c1c9fd8d852bc96e1e7394de34ff262fb6a40c47b1e0a32c897","nonceHex":"7f18dfd47818c99988354959","payloadCiphertextHex":"4a4824822587091c18bc17a59683982d465f4627488db64774ae2f8c3f7ace39b497311e7b0fb47b43872d82d8cb14ce205d1960cb3df19f92248add1ad1b7849ca825120a0c74d919d4f6d44e5f7e9f7d536f058afde4c5c5cb79b9386f2edcbc8c1ebd22a529e6b062c91504231761493e2eadcdbb306c0adc533f55e0e390b6e2ed720f49d8bfa6b59a7ddfd562c43eb467d14e3b2e8483ff511aa30970cda8a16f5e7d583941ed2c0d7db54e68cd239a6d832ea1819c8ee8584f00f2881a6a87ad16e70840f8362b047544cc6efd48ce6be4ce9d01661959d25f6222ebc06013f33e143ba8279dde228bc3c257f95091ee6c1cb204e96297fc86e093e6fc286c073ad8aaa71fabc9f215958725204c1094c4c5a6f5a455647e31212142dd002acd0d0cfd8523947cad3d195e13fd949abb907f57b320628462666a3f79e3122ed05beb2156607e33805e9b89e6ae26d5f9cf428698616f7e37b1ca91a38435eb40c6954617c30df9bee8aa16dcc20a250b274c3e054d7384cd11f65bb6a233"}"#;

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("valid hex"))
        .collect()
}

fn dse1_frame(blob: &[u8]) -> Vec<u8> {
    dossier_contract::envelope::frame(blob)
}

#[test]
fn shell_registers_real_enclave_key_and_admits_hybrid_envelope() {
    let pubkey_bytes = hex_to_bytes(REAL_PUBKEY_HEX);
    assert_eq!(pubkey_bytes.len(), 48, "pubkey must be a compressed G1 point");
    let mut pubkey = [0u8; 48];
    pubkey.copy_from_slice(&pubkey_bytes);

    let (mut deps, env, owner, _orch, _v) = setup_with_pubkey(pubkey);

    // The registered key is queryable.
    let stored: Option<Vec<u8>> =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::EnclavePubkey {}).unwrap()).unwrap();
    assert_eq!(stored.as_deref(), Some(&pubkey_bytes[..]));

    // Frame the hybrid envelope JSON so it passes the contract's structural
    // DSE1 check; then store it via Admit.
    let framed_ct = dse1_frame(REAL_ENVELOPE_JSON.as_bytes());
    assert!(dossier_contract::envelope::well_formed(&framed_ct));

    let resp = execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::Admit {
            schema_id: "schema-a".into(),
            ciphertext: framed_ct.clone(),
            proof_blob: b"proof".to_vec(),
            provenance: Provenance {
                attestor: "bank-x".into(),
                proof_type: ProofType::ZkTls,
            },
        },
    )
    .unwrap();

    let admission_id: u64 = resp
        .attributes
        .iter()
        .find(|a| a.key == "admission_id")
        .unwrap()
        .value
        .parse()
        .unwrap();

    let state: DossierState =
        from_json(query(deps.as_ref(), env.clone(), QueryMsg::State {}).unwrap()).unwrap();
    let pending = &state.pending_admissions[&admission_id];
    assert_eq!(pending.ciphertext, framed_ct);
    assert_eq!(pending.schema_id, "schema-a");
    assert_eq!(pending.provenance.attestor, "bank-x");
}

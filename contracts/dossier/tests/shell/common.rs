//! Shared fixtures + helpers for the shell integration tests.

pub(crate) use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
pub(crate) use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
pub(crate) use cosmwasm_std::{from_json, Empty, OwnedDeps, Timestamp};

pub(crate) use dossier_contract::contract::{execute, instantiate, query};
pub(crate) use dossier_contract::empty_entries_root;
pub(crate) use dossier_contract::machine::mock::MockVerifier;
pub(crate) use dossier_contract::machine::{Binding, DossierState, KeyBinding, KeyPurpose};
pub(crate) use dossier_contract::merkle::{self, sha256};
pub(crate) use dossier_contract::msg::{
    AdmissionRejectReason, ExecuteMsg, InstantiateMsg, QueryMsg,
};
pub(crate) use dossier_contract::state::{
    dossier_derivation_path, DisclosureOutcome, FailReason, Hash32, ProofType, Provenance,
    KEY_ROTATION_TIMELOCK_SECONDS, RETENTION_WINDOW_SECONDS,
};

pub(crate) const T0: u64 = 1_700_000_000;

/// Deterministic envelope + pubkey-padding helpers (canonical impls live in
/// `dossier_protocol::testing`, shared with the other test suites).
pub(crate) use dossier_protocol::testing::{pad48, seeded_envelope as ev};
/// Set up a fresh contract with a registered enclave key.
pub(crate) type Deps = OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>;

pub(crate) fn setup_with_pubkey(pubkey: [u8; 48]) -> (
    Deps,
    cosmwasm_std::Env,
    cosmwasm_std::Addr,
    cosmwasm_std::Addr,
    MockVerifier,
) {
    let mut deps = mock_dependencies();
    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(T0);
    let owner = deps.api.addr_make("alice");
    let orch = deps.api.addr_make("orchestrator");
    let v = MockVerifier { epoch: 1 };

    instantiate(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        InstantiateMsg {
            snapshot_retention_floor_seconds: 1_000,
            schema_registry: vec!["schema-a".into()],
            vkey_name: "zkdcap-v1".into(),
            min_tcb_eval_num: 0,
        },
    )
    .unwrap();

    let contract_addr = env.contract.address.to_string();
    let path = dossier_derivation_path(&contract_addr);
    let key_att = v.attest_key(
        &KeyBinding {
            pubkey: &pubkey,
            contract_address: &contract_addr,
            derivation_path: &path,
            purpose: KeyPurpose::Register,
        },
        T0,
    );
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(&owner, &[]),
        ExecuteMsg::RegisterEnclaveKey {
            pubkey: pubkey.to_vec(),
            attestation: key_att,
        },
    )
    .unwrap();

    (deps, env, owner, orch, v)
}

pub(crate) fn setup_test() -> (
    Deps,
    cosmwasm_std::Env,
    cosmwasm_std::Addr,
    cosmwasm_std::Addr,
    MockVerifier,
) {
    let bytes = pad48(b"pk-enclave");
    let mut arr = [0u8; 48];
    arr.copy_from_slice(&bytes);
    setup_with_pubkey(arr)
}

pub(crate) fn admit_id(
    deps: &mut Deps,
    env: &cosmwasm_std::Env,
    owner: &cosmwasm_std::Addr,
    ct_seed: &[u8],
) -> u64 {
    let resp = execute(
        deps.as_mut(),
        env.clone(),
        message_info(owner, &[]),
        ExecuteMsg::Admit {
            schema_id: "schema-a".into(),
            ciphertext: ev(ct_seed),
            proof_blob: b"proof".to_vec(),
            provenance: Provenance {
                attestor: "bank-x".into(),
                proof_type: ProofType::ZkTls,
            },
        },
    )
    .unwrap();
    resp.attributes
        .iter()
        .find(|a| a.key == "admission_id")
        .unwrap()
        .value
        .parse()
        .unwrap()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn accept(
    deps: &mut Deps,
    env: &cosmwasm_std::Env,
    orch: &cosmwasm_std::Addr,
    v: &MockVerifier,
    contract_addr: &str,
    chain_id: &str,
    admission_id: u64,
    ct_seed: &[u8],
) {
    let att = v.attest(&Binding::AdmissionAccept {
        chain_id,
        contract_address: contract_addr,
        admission_id,
        ciphertext_hash: sha256(&ev(ct_seed)),
        schema_id: "schema-a",
        enclave_pubkey_hash: sha256(&pad48(b"pk-enclave")),
    });
    execute(
        deps.as_mut(),
        env.clone(),
        message_info(orch, &[]),
        ExecuteMsg::AdmissionAccept {
            admission_id,
            attestation: att,
        },
    )
    .unwrap();
}

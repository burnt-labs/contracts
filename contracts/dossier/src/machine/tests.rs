use std::collections::BTreeSet;

use super::mock::*;
use super::*;
use crate::state::ProofType;
use dossier_protocol::testing::{pad48, seeded_envelope as ev};

const OWNER: &str = "alice";
const ADDR: &str = "xion1dossier";
const CHAIN: &str = "dossier-test-1";
const FLOOR: u64 = 1_000;
const T0: u64 = 10_000_000;


fn setup() -> (DossierState, MockVerifier) {
    let mut reg = BTreeSet::new();
    reg.insert("schema-a".to_string());
    let st = DossierState::new(
        OWNER.into(),
        ADDR.into(),
        Config {
            snapshot_retention_floor_seconds: FLOOR,
            schema_registry: reg,
            vkey_name: "zkdcap-v1".into(),
            min_tcb_eval_num: 0,
        },
    );
    (st, MockVerifier { epoch: 1 })
}

fn env(now: u64, sender: &str) -> Env {
    Env {
        now,
        sender: sender.into(),
        chain_id: CHAIN.into(),
    }
}

fn prov() -> Provenance {
    Provenance {
        attestor: "attestor-x".into(),
        proof_type: ProofType::ZkTls,
    }
}

fn key_att(v: &MockVerifier, pk: &[u8], t: u64, purpose: KeyPurpose) -> Vec<u8> {
    let path = dossier_derivation_path(ADDR);
    v.attest_key(
        &KeyBinding {
            pubkey: pk,
            contract_address: ADDR,
            derivation_path: &path,
            purpose,
        },
        t,
    )
}

fn register(st: &mut DossierState, v: &MockVerifier, now: u64, pk: &[u8]) {
    st.register_enclave_key(
        v,
        &env(now, OWNER),
        pk.to_vec(),
        key_att(v, pk, now, KeyPurpose::Register),
    )
    .unwrap();
}

fn admit(st: &mut DossierState, now: u64, ct: &[u8]) -> Id {
    st.admit(
        &env(now, OWNER),
        "schema-a".into(),
        ev(ct),
        vec![0xAB; 4],
        prov(),
    )
    .unwrap()
}

fn accept(st: &mut DossierState, v: &MockVerifier, now: u64, id: Id, _ct: &[u8]) {
    let pkh = sha256(st.enclave_pubkey.as_deref().unwrap());
    let b = Binding::AdmissionAccept {
        chain_id: CHAIN,
        contract_address: ADDR,
        admission_id: id,
        ciphertext_hash: sha256(&st.pending_admissions[&id].ciphertext),
        schema_id: "schema-a",
        enclave_pubkey_hash: pkh,
    };
    let att = v.attest(&b);
    st.admission_accept(v, &env(now, "orchestrator"), id, att)
        .unwrap();
}

fn rotate(st: &mut DossierState, v: &MockVerifier, now: u64, new_pk: &[u8]) -> u64 {
    st.propose_key_rotation(
        v,
        &env(now, OWNER),
        new_pk.to_vec(),
        key_att(v, new_pk, now, KeyPurpose::Rotate),
    )
    .unwrap();
    let after = now + KEY_ROTATION_TIMELOCK_SECONDS;
    st.finalize_key_rotation(v, &env(after, "anyone")).unwrap();
    after
}

fn fulfil_binding(
    st: &DossierState,
    id: Id,
    disclosure_ct: &[u8],
    pk_c_hash: Hash32,
) -> (Binding<'static>, Hash32) {
    let pd = &st.pending_disclosures[&id];
    let root = pd.entries_root_at_create;
    (
        Binding::Fulfil {
            chain_id: CHAIN,
            contract_address: ADDR,
            disclosure_id: id,
            request_blob_hash: sha256(&pd.encrypted_request_blob),
            disclosure_ciphertext_hash: sha256(disclosure_ct),
            snapshot_root: root,
            pk_c_hash,
            enclave_pubkey_hash: sha256(st.enclave_pubkey.as_deref().unwrap()),
        },
        root,
    )
}

fn fail_att(
    st: &DossierState,
    v: &MockVerifier,
    id: Id,
    reason: FailReason,
    fail_ct: &[u8],
    snapshot_root: Hash32,
    pk_slot: Hash32,
) -> Vec<u8> {
    let pd = &st.pending_disclosures[&id];
    let b = Binding::Fail {
        chain_id: CHAIN,
        contract_address: ADDR,
        disclosure_id: id,
        reason,
        request_blob_hash: sha256(&pd.encrypted_request_blob),
        fail_reason_ciphertext_hash: sha256(fail_ct),
        snapshot_root,
        pk_slot,
        enclave_pubkey_hash: sha256(st.enclave_pubkey.as_deref().unwrap()),
    };
    v.attest(&b)
}

#[test]
fn d1_register_then_already_set() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    assert_eq!(st.enclave_pubkey.as_deref(), Some(&pad48(b"pk-old")[..]));
    let err = st
        .register_enclave_key(
            &v,
            &env(T0, OWNER),
            pad48(b"pk-2"),
            key_att(&v, &pad48(b"pk-2"), T0, KeyPurpose::Register),
        )
        .unwrap_err();
    assert_eq!(err, RejectCode::KeyAlreadySet);
}

#[test]
fn d1_d2_reject_malformed_pubkey() {
    let (mut st, v) = setup();
    let short = vec![7u8; 47];
    assert_eq!(
        st.register_enclave_key(
            &v,
            &env(T0, OWNER),
            short.clone(),
            key_att(&v, &short, T0, KeyPurpose::Register)
        ),
        Err(RejectCode::MalformedEnclaveKey)
    );
    // a well-formed 48-byte key registers
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    // D2 to a wrong-length key is rejected too
    let long = vec![9u8; 49];
    assert_eq!(
        st.propose_key_rotation(
            &v,
            &env(T0, OWNER),
            long.clone(),
            key_att(&v, &long, T0, KeyPurpose::Rotate)
        ),
        Err(RejectCode::MalformedEnclaveKey)
    );
}

#[test]
fn config_floor_bounds() {
    use crate::state::MAX_SNAPSHOT_RETENTION_FLOOR_SECONDS as MAX;
    let mk = |floor| {
        let mut reg = BTreeSet::new();
        reg.insert("schema-a".to_string());
        Config {
            snapshot_retention_floor_seconds: floor,
            schema_registry: reg,
            vkey_name: "v".into(),
            min_tcb_eval_num: 0,
        }
    };
    assert!(!mk(0).validate());
    assert!(mk(1).validate());
    assert!(mk(MAX).validate());
    assert!(!mk(MAX + 1).validate());
}

#[test]
fn d1_rejects_wrong_binding() {
    let (mut st, v) = setup();
    let other = v.attest_key(
        &KeyBinding {
            pubkey: &pad48(b"pk"),
            contract_address: "xion1other",
            derivation_path: "dossier-v1:xion1other",
            purpose: KeyPurpose::Register,
        },
        T0,
    );
    assert_eq!(
        st.register_enclave_key(&v, &env(T0, OWNER), pad48(b"pk"), other),
        Err(RejectCode::DossierBindingMismatch)
    );
}

#[test]
fn d2_rejects_identical_key_and_blocks_a1_c1() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    assert_eq!(
        st.propose_key_rotation(
            &v,
            &env(T0, OWNER),
            pad48(b"pk-old"),
            key_att(&v, &pad48(b"pk-old"), T0, KeyPurpose::Rotate)
        ),
        Err(RejectCode::IdenticalKey)
    );
    st.propose_key_rotation(
        &v,
        &env(T0, OWNER),
        pad48(b"pk-new"),
        key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate),
    )
    .unwrap();
    assert_eq!(
        st.admit(&env(T0, OWNER), "schema-a".into(), vec![1], vec![1], prov()),
        Err(RejectCode::PendingRotation)
    );
    assert_eq!(
        st.create_disclosure(&env(T0, OWNER), vec![1]),
        Err(RejectCode::PendingRotation)
    );
}

#[test]
fn d3_timelock_then_promotes() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    st.propose_key_rotation(
        &v,
        &env(T0, OWNER),
        pad48(b"pk-new"),
        key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate),
    )
    .unwrap();
    assert_eq!(
        st.finalize_key_rotation(&v, &env(T0 + 1, "anyone")),
        Err(RejectCode::TimelockNotElapsed)
    );
    st.finalize_key_rotation(&v, &env(T0 + KEY_ROTATION_TIMELOCK_SECONDS, "anyone"))
        .unwrap();
    assert_eq!(st.enclave_pubkey.as_deref(), Some(&pad48(b"pk-new")[..]));
    assert!(st.pending_enclave_key.is_none());
    assert_eq!(
        st.finalize_key_rotation(&v, &env(T0 + KEY_ROTATION_TIMELOCK_SECONDS, "anyone")),
        Err(RejectCode::NoRotationPending)
    );
}

#[test]
fn d3_rejects_after_vkey_rotation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    st.propose_key_rotation(
        &v,
        &env(T0, OWNER),
        pad48(b"pk-new"),
        key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate),
    )
    .unwrap();
    let rotated_vkey = MockVerifier { epoch: 2 };
    assert_eq!(
        st.finalize_key_rotation(&rotated_vkey, &env(T0 + KEY_ROTATION_TIMELOCK_SECONDS, "x")),
        Err(RejectCode::AttestationExpired)
    );
    // owner cancels and re-proposes under the new vkey
    st.cancel_key_rotation(&env(T0, OWNER)).unwrap();
    assert!(st.pending_enclave_key.is_none());
}

// ---- block A ----

#[test]
fn a1_assigns_monotonic_ids_and_validates() {
    let (mut st, v) = setup();
    assert_eq!(
        st.admit(&env(T0, OWNER), "schema-a".into(), vec![1], vec![1], prov()),
        Err(RejectCode::NoKeySet)
    );
    register(&mut st, &v, T0, &pad48(b"pk"));
    // A1 is permissionless: a non-owner sender stages an admission too.
    let id0 = st
        .admit(
            &env(T0, "bob"),
            "schema-a".into(),
            ev(b"ct-0"),
            vec![1],
            prov(),
        )
        .unwrap();
    assert_eq!(
        st.admit(&env(T0, OWNER), "schema-z".into(), vec![1], vec![1], prov()),
        Err(RejectCode::UnsupportedSchema)
    );
    let id1 = admit(&mut st, T0, b"ct-1");
    let id2 = admit(&mut st, T0, b"ct-2");
    assert_eq!((id0, id1, id2), (1, 2, 3));
}

#[test]
fn a1_c1_reject_unframed_envelopes() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    assert_eq!(
        st.admit(
            &env(T0, OWNER),
            "schema-a".into(),
            b"raw".to_vec(),
            vec![1],
            prov()
        ),
        Err(RejectCode::InvalidEnvelope)
    );
    assert_eq!(
        st.create_disclosure(&env(T0, OWNER), b"raw".to_vec()),
        Err(RejectCode::InvalidEnvelope)
    );
}

#[test]
fn a2_inherits_id_and_recomputes_root() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id = admit(&mut st, T0, b"ct-1");
    assert_eq!(st.entries_root, merkle::empty_entries_root());
    accept(&mut st, &v, T0, id, b"ct-1");
    assert!(st.entries.contains_key(&id));
    assert!(st.pending_admissions.is_empty());
    assert_eq!(st.entries_root, merkle::entries_root_of_map(&st.entries));
    assert_ne!(st.entries_root, merkle::empty_entries_root());
}

#[test]
fn a2_drain_or_fail_after_rotation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    let id = admit(&mut st, T0, b"ct-1");
    let ct_hash = sha256(&st.pending_admissions[&id].ciphertext);
    // enclave processed under pk-old and minted the attestation...
    let old_binding = Binding::AdmissionAccept {
        chain_id: CHAIN,
        contract_address: ADDR,
        admission_id: id,
        ciphertext_hash: ct_hash,
        schema_id: "schema-a",
        enclave_pubkey_hash: sha256(&pad48(b"pk-old")),
    };
    let old_att = v.attest(&old_binding);
    // ...but rotation lands first (drain-or-fail)
    let after = rotate(&mut st, &v, T0, &pad48(b"pk-new"));
    assert_eq!(
        st.admission_accept(&v, &env(after, "orch"), id, old_att),
        Err(RejectCode::KeyMismatch)
    );
    // new-key enclave fail-finalizes the straggler
    let reject_binding = Binding::AdmissionReject {
        chain_id: CHAIN,
        contract_address: ADDR,
        admission_id: id,
        reason: AdmissionRejectReason::StaleEnclaveKey,
        ciphertext_hash: ct_hash,
        schema_id: "schema-a",
        enclave_pubkey_hash: sha256(&pad48(b"pk-new")),
    };
    let att = v.attest(&reject_binding);
    st.admission_reject(
        &v,
        &env(after, "orch"),
        id,
        AdmissionRejectReason::StaleEnclaveKey,
        att,
    )
    .unwrap();
    assert!(st.pending_admissions.is_empty());
    assert!(st.entries.is_empty());
}

#[test]
fn a2_rejects_tampered_binding() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id = admit(&mut st, T0, b"ct-1");
    // attestation minted over a DIFFERENT ciphertext hash
    let b = Binding::AdmissionAccept {
        chain_id: CHAIN,
        contract_address: ADDR,
        admission_id: id,
        ciphertext_hash: sha256(b"ct-OTHER"),
        schema_id: "schema-a",
        enclave_pubkey_hash: sha256(&pad48(b"pk")),
    };
    let att = v.attest(&b);
    assert_eq!(
        st.admission_accept(&v, &env(T0, "orch"), id, att),
        Err(RejectCode::InvalidAttestation)
    );
    assert_eq!(
        st.admission_accept(&v, &env(T0, "orch"), 99, vec![0x01]),
        Err(RejectCode::NotFound)
    );
}

#[test]
fn a4_owner_cancel() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id = admit(&mut st, T0, b"ct-1");
    assert_eq!(
        st.cancel_pending_admission(&env(T0, "bob"), id),
        Err(RejectCode::Unauthorized)
    );
    st.cancel_pending_admission(&env(T0, OWNER), id).unwrap();
    assert_eq!(
        st.cancel_pending_admission(&env(T0, OWNER), id),
        Err(RejectCode::NotFound)
    );
}

// ---- block B ----

#[test]
fn revoke_recomputes_root() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id1 = admit(&mut st, T0, b"ct-1");
    accept(&mut st, &v, T0, id1, b"ct-1");
    let id2 = admit(&mut st, T0, b"ct-2");
    accept(&mut st, &v, T0, id2, b"ct-2");
    st.revoke(&env(T0, OWNER), id1).unwrap();
    assert_eq!(st.entries_root, merkle::entries_root_of_map(&st.entries));
    assert_eq!(st.entries.len(), 1);
    assert_eq!(st.revoke(&env(T0, OWNER), id1), Err(RejectCode::NotFound));
}

// ---- block C ----

#[test]
fn c1_pins_root_at_create() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let e1 = admit(&mut st, T0, b"ct-1");
    accept(&mut st, &v, T0, e1, b"ct-1");
    let pinned = st.entries_root;
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    // later mutation does not move the pin (snapshot-at-approval)
    let e2 = admit(&mut st, T0, b"ct-2");
    accept(&mut st, &v, T0, e2, b"ct-2");
    assert_ne!(st.entries_root, pinned);
    assert_eq!(st.pending_disclosures[&did].entries_root_at_create, pinned);
}

#[test]
fn c2_fulfil_and_snapshot_mismatch() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let e1 = admit(&mut st, T0, b"ct-1");
    accept(&mut st, &v, T0, e1, b"ct-1");
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let (binding, root) = fulfil_binding(&st, did, b"result", sha256(b"pk_C"));
    // wrong snapshot_root first
    let att = v.attest(&binding);
    assert_eq!(
        st.fulfil_disclosure(
            &v,
            &env(T0, "orch"),
            did,
            b"result".to_vec(),
            [9u8; 32],
            sha256(b"pk_C"),
            att.clone()
        ),
        Err(RejectCode::SnapshotMismatch)
    );
    st.fulfil_disclosure(
        &v,
        &env(T0 + 5, "orch"),
        did,
        b"result".to_vec(),
        root,
        sha256(b"pk_C"),
        att,
    )
    .unwrap();
    let fd = &st.disclosures[&did];
    assert_eq!(fd.snapshot_root, root);
    assert_eq!(fd.created_ts, T0);
    assert_eq!(fd.finalized_ts, T0 + 5);
    assert!(matches!(fd.outcome, DisclosureOutcome::Fulfilled { .. }));
}

#[test]
fn c2_drain_or_fail_after_rotation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let (old_binding, root) = fulfil_binding(&st, did, b"result", sha256(b"pk_C"));
    let old_att = v.attest(&old_binding);
    let after = rotate(&mut st, &v, T0, &pad48(b"pk-new"));
    assert_eq!(
        st.fulfil_disclosure(
            &v,
            &env(after, "orch"),
            did,
            b"result".to_vec(),
            root,
            sha256(b"pk_C"),
            old_att
        ),
        Err(RejectCode::KeyMismatch)
    );
    // new-key enclave fail-finalizes with request_decryption_failed
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::RequestDecryptionFailed,
        b"",
        root,
        sentinel_pk_unrecoverable(),
    );
    st.fail_disclosure(
        &v,
        &env(after, "orch"),
        did,
        FailReason::RequestDecryptionFailed,
        vec![],
        root,
        sentinel_pk_unrecoverable(),
        att,
    )
    .unwrap();
    let fd = &st.disclosures[&did];
    assert!(matches!(
        fd.outcome,
        DisclosureOutcome::Failed {
            reason: FailReason::RequestDecryptionFailed,
            ..
        }
    ));
}

#[test]
fn c3_sentinel_rules() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let e1 = admit(&mut st, T0, b"ct-1");
    accept(&mut st, &v, T0, e1, b"ct-1");
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let pinned = st.pending_disclosures[&did].entries_root_at_create;
    let sentinel = sentinel_snapshot_unavailable();
    let pk_c = sha256(b"pk_C");

    // sentinel root with a non-sentinel reason
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::PredicateUndefined,
        b"detail",
        sentinel,
        pk_c,
    );
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0 + FLOOR + 1, "orch"),
            did,
            FailReason::PredicateUndefined,
            b"detail".to_vec(),
            sentinel,
            pk_c,
            att
        ),
        Err(RejectCode::SnapshotMismatch)
    );
    // snapshot_unavailable with the pinned (non-sentinel) root
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::SnapshotUnavailable,
        b"detail",
        pinned,
        pk_c,
    );
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0 + FLOOR + 1, "orch"),
            did,
            FailReason::SnapshotUnavailable,
            b"detail".to_vec(),
            pinned,
            pk_c,
            att
        ),
        Err(RejectCode::SnapshotMismatch)
    );
    // premature sentinel (retention floor not elapsed)
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::SnapshotUnavailable,
        b"detail",
        sentinel,
        pk_c,
    );
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0 + FLOOR - 1, "orch"),
            did,
            FailReason::SnapshotUnavailable,
            b"detail".to_vec(),
            sentinel,
            pk_c,
            att.clone()
        ),
        Err(RejectCode::SnapshotNotOldEnough)
    );
    // post-floor sentinel accepted
    st.fail_disclosure(
        &v,
        &env(T0 + FLOOR, "orch"),
        did,
        FailReason::SnapshotUnavailable,
        b"detail".to_vec(),
        sentinel,
        pk_c,
        att,
    )
    .unwrap();
    assert_eq!(st.disclosures[&did].snapshot_root, sentinel);
}

#[test]
fn c3_sentinel_rejected_for_empty_dossier() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    // empty dossier: pinned root is the empty-entries root
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let sentinel = sentinel_snapshot_unavailable();
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::SnapshotUnavailable,
        b"",
        sentinel,
        sha256(b"pk_C"),
    );
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0 + FLOOR + 1, "orch"),
            did,
            FailReason::SnapshotUnavailable,
            vec![],
            sentinel,
            sha256(b"pk_C"),
            att
        ),
        Err(RejectCode::SnapshotMismatch)
    );
}

#[test]
fn c3_pk_slot_reason_consistency() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let pinned = st.pending_disclosures[&did].entries_root_at_create;

    // RDF with non-empty payload
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::RequestDecryptionFailed,
        b"detail",
        pinned,
        sentinel_pk_unrecoverable(),
    );
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0, "orch"),
            did,
            FailReason::RequestDecryptionFailed,
            b"detail".to_vec(),
            pinned,
            sentinel_pk_unrecoverable(),
            att
        ),
        Err(RejectCode::InvalidAttestation)
    );
    // RDF without the sentinel pk slot
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::RequestDecryptionFailed,
        b"",
        pinned,
        sha256(b"pk_C"),
    );
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0, "orch"),
            did,
            FailReason::RequestDecryptionFailed,
            vec![],
            pinned,
            sha256(b"pk_C"),
            att
        ),
        Err(RejectCode::InvalidAttestation)
    );
    // non-RDF with the sentinel pk slot
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::OtherEnclaveError,
        b"detail",
        pinned,
        sentinel_pk_unrecoverable(),
    );
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0, "orch"),
            did,
            FailReason::OtherEnclaveError,
            b"detail".to_vec(),
            pinned,
            sentinel_pk_unrecoverable(),
            att
        ),
        Err(RejectCode::InvalidAttestation)
    );
    // well-formed RDF
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::RequestDecryptionFailed,
        b"",
        pinned,
        sentinel_pk_unrecoverable(),
    );
    st.fail_disclosure(
        &v,
        &env(T0, "orch"),
        did,
        FailReason::RequestDecryptionFailed,
        vec![],
        pinned,
        sentinel_pk_unrecoverable(),
        att,
    )
    .unwrap();
}

#[test]
fn c5_retention_window() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let (binding, root) = fulfil_binding(&st, did, b"result", sha256(b"pk_C"));
    let att = v.attest(&binding);
    st.fulfil_disclosure(
        &v,
        &env(T0, "orch"),
        did,
        b"result".to_vec(),
        root,
        sha256(b"pk_C"),
        att,
    )
    .unwrap();
    assert_eq!(
        st.prune_disclosure(&env(T0 + RETENTION_WINDOW_SECONDS - 1, "anyone"), did),
        Err(RejectCode::RetentionWindowNotElapsed)
    );
    st.prune_disclosure(&env(T0 + RETENTION_WINDOW_SECONDS, "anyone"), did)
        .unwrap();
    assert!(st.disclosures.is_empty());
}

// ---- missing D1 paths ----

#[test]
fn d1_rejects_non_owner() {
    let (mut st, v) = setup();
    assert_eq!(
        st.register_enclave_key(
            &v,
            &env(T0, "bob"),
            pad48(b"pk"),
            key_att(&v, &pad48(b"pk"), T0, KeyPurpose::Register)
        ),
        Err(RejectCode::Unauthorized)
    );
}

#[test]
fn d1_rejects_garbage_attestation() {
    let (mut st, v) = setup();
    assert_eq!(
        st.register_enclave_key(&v, &env(T0, OWNER), pad48(b"pk"), vec![0x00, 0x01]),
        Err(RejectCode::InvalidAttestation)
    );
}

// ---- missing D2 paths ----

#[test]
fn d2_rejects_non_owner() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    assert_eq!(
        st.propose_key_rotation(
            &v,
            &env(T0, "bob"),
            pad48(b"pk-new"),
            key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate)
        ),
        Err(RejectCode::Unauthorized)
    );
}

#[test]
fn d2_rejects_no_key_set() {
    let (mut st, v) = setup();
    assert_eq!(
        st.propose_key_rotation(
            &v,
            &env(T0, OWNER),
            pad48(b"pk-new"),
            key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate)
        ),
        Err(RejectCode::NoKeySet)
    );
}

#[test]
fn d2_rejects_garbage_attestation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    assert_eq!(
        st.propose_key_rotation(&v, &env(T0, OWNER), pad48(b"pk-new"), vec![0x00]),
        Err(RejectCode::InvalidAttestation)
    );
}

#[test]
fn d2_rejects_binding_mismatch() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    let other = v.attest_key(
        &KeyBinding {
            pubkey: &pad48(b"pk-new"),
            contract_address: "xion1other",
            derivation_path: "dossier-v1:xion1other",
            purpose: KeyPurpose::Rotate,
        },
        T0,
    );
    assert_eq!(
        st.propose_key_rotation(&v, &env(T0, OWNER), pad48(b"pk-new"), other),
        Err(RejectCode::DossierBindingMismatch)
    );
}

#[test]
fn d2_accepts_old_host_timestamp() {
    // Freshness moved off the untrusted host timestamp (now the circuit's
    // validity window + tcb_eval floor, enforced in the verifier), so an old
    // attestation timestamp no longer rejects a rotation proposal.
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    let att = key_att(&v, &pad48(b"pk-new"), T0 - 100_000, KeyPurpose::Rotate);
    assert!(st
        .propose_key_rotation(&v, &env(T0, OWNER), pad48(b"pk-new"), att)
        .is_ok());
}

// ---- D4 paths (all missing) ----

#[test]
fn d4_rejects_non_owner() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    st.propose_key_rotation(
        &v,
        &env(T0, OWNER),
        pad48(b"pk-new"),
        key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate),
    )
    .unwrap();
    assert_eq!(
        st.cancel_key_rotation(&env(T0, "bob")),
        Err(RejectCode::Unauthorized)
    );
}

#[test]
fn d4_rejects_no_pending() {
    let (mut st, _v) = setup();
    assert_eq!(
        st.cancel_key_rotation(&env(T0, OWNER)),
        Err(RejectCode::NoRotationPending)
    );
}

#[test]
fn d4_owner_cancels() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    st.propose_key_rotation(
        &v,
        &env(T0, OWNER),
        pad48(b"pk-new"),
        key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate),
    )
    .unwrap();
    assert!(st.pending_enclave_key.is_some());
    st.cancel_key_rotation(&env(T0, OWNER)).unwrap();
    assert!(st.pending_enclave_key.is_none());
    assert_eq!(st.enclave_pubkey.as_deref(), Some(&pad48(b"pk-old")[..]));
}

// ---- missing A1 paths ----

#[test]
fn a1_rejects_empty_proof() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    assert_eq!(
        st.admit(
            &env(T0, OWNER),
            "schema-a".into(),
            ev(b"ct"),
            vec![],
            prov()
        ),
        Err(RejectCode::InvalidProofFormat)
    );
}

#[test]
fn a1_rejects_pending_rotation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    st.propose_key_rotation(
        &v,
        &env(T0, OWNER),
        pad48(b"pk-new"),
        key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate),
    )
    .unwrap();
    assert_eq!(
        st.admit(
            &env(T0, OWNER),
            "schema-a".into(),
            ev(b"ct"),
            vec![1],
            prov()
        ),
        Err(RejectCode::PendingRotation)
    );
}

// ---- A3 paths (all missing) ----

#[test]
fn a3_not_found() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    assert_eq!(
        st.admission_reject(
            &v,
            &env(T0, "orch"),
            99,
            AdmissionRejectReason::InvalidProof,
            vec![0x01]
        ),
        Err(RejectCode::NotFound)
    );
}

#[test]
fn a3_no_key_set() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id = admit(&mut st, T0, b"ct-1");
    st.enclave_pubkey = None;
    assert_eq!(
        st.admission_reject(
            &v,
            &env(T0, "orch"),
            id,
            AdmissionRejectReason::InvalidProof,
            vec![0x01]
        ),
        Err(RejectCode::NoKeySet)
    );
}

#[test]
fn a3_rejects_garbage_attestation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id = admit(&mut st, T0, b"ct-1");
    assert_eq!(
        st.admission_reject(
            &v,
            &env(T0, "orch"),
            id,
            AdmissionRejectReason::InvalidProof,
            vec![0x00, 0xEE]
        ),
        Err(RejectCode::InvalidAttestation)
    );
}

#[test]
fn a3_key_mismatch_after_rotation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    let id = admit(&mut st, T0, b"ct-1");
    let ct_hash = sha256(&st.pending_admissions[&id].ciphertext);
    let old_binding = Binding::AdmissionReject {
        chain_id: CHAIN,
        contract_address: ADDR,
        admission_id: id,
        reason: AdmissionRejectReason::InvalidProof,
        ciphertext_hash: ct_hash,
        schema_id: "schema-a",
        enclave_pubkey_hash: sha256(&pad48(b"pk-old")),
    };
    let old_att = v.attest(&old_binding);
    let after = rotate(&mut st, &v, T0, &pad48(b"pk-new"));
    assert_eq!(
        st.admission_reject(
            &v,
            &env(after, "orch"),
            id,
            AdmissionRejectReason::InvalidProof,
            old_att
        ),
        Err(RejectCode::KeyMismatch)
    );
}

#[test]
fn a3_accepts_valid_rejection() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id = admit(&mut st, T0, b"ct-1");
    let pk_hash = sha256(&pad48(b"pk"));
    let ct_hash = sha256(&st.pending_admissions[&id].ciphertext);
    let binding = Binding::AdmissionReject {
        chain_id: CHAIN,
        contract_address: ADDR,
        admission_id: id,
        reason: AdmissionRejectReason::SchemaMismatch,
        ciphertext_hash: ct_hash,
        schema_id: "schema-a",
        enclave_pubkey_hash: pk_hash,
    };
    let att = v.attest(&binding);
    st.admission_reject(
        &v,
        &env(T0, "orch"),
        id,
        AdmissionRejectReason::SchemaMismatch,
        att,
    )
    .unwrap();
    assert!(st.pending_admissions.is_empty());
}

// ---- missing B paths ----

#[test]
fn revoke_rejects_non_owner() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let id = admit(&mut st, T0, b"ct-1");
    accept(&mut st, &v, T0, id, b"ct-1");
    assert_eq!(
        st.revoke(&env(T0, "bob"), id),
        Err(RejectCode::Unauthorized)
    );
}

// ---- missing C1 paths ----

#[test]
fn c1_rejects_non_owner() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    assert_eq!(
        st.create_disclosure(&env(T0, "bob"), ev(b"req")),
        Err(RejectCode::Unauthorized)
    );
}

#[test]
fn c1_rejects_no_key_set() {
    let (mut st, _v) = setup();
    assert_eq!(
        st.create_disclosure(&env(T0, OWNER), ev(b"req")),
        Err(RejectCode::NoKeySet)
    );
}

#[test]
fn c1_rejects_pending_rotation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    st.propose_key_rotation(
        &v,
        &env(T0, OWNER),
        pad48(b"pk-new"),
        key_att(&v, &pad48(b"pk-new"), T0, KeyPurpose::Rotate),
    )
    .unwrap();
    assert_eq!(
        st.create_disclosure(&env(T0, OWNER), ev(b"req")),
        Err(RejectCode::PendingRotation)
    );
}

// ---- missing C2 paths ----

#[test]
fn c2_not_found() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    assert_eq!(
        st.fulfil_disclosure(
            &v,
            &env(T0, "orch"),
            99,
            vec![0xD0],
            [0u8; 32],
            sha256(b"pk_C"),
            vec![0x01]
        ),
        Err(RejectCode::NotFound)
    );
}

#[test]
fn c2_no_key_set() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    st.enclave_pubkey = None;
    assert_eq!(
        st.fulfil_disclosure(
            &v,
            &env(T0, "orch"),
            did,
            vec![0xD0],
            [0u8; 32],
            sha256(b"pk_C"),
            vec![0x01]
        ),
        Err(RejectCode::NoKeySet)
    );
}

#[test]
fn c2_rejects_garbage_attestation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let (_, root) = fulfil_binding(&st, did, b"result", sha256(b"pk_C"));
    assert_eq!(
        st.fulfil_disclosure(
            &v,
            &env(T0, "orch"),
            did,
            b"result".to_vec(),
            root,
            sha256(b"pk_C"),
            vec![0x01, 0xEE]
        ),
        Err(RejectCode::InvalidAttestation)
    );
}

// ---- missing C3 paths ----

#[test]
fn c3_not_found() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0, "orch"),
            99,
            FailReason::OtherEnclaveError,
            vec![],
            [0u8; 32],
            sha256(b"pk_C"),
            vec![0x01]
        ),
        Err(RejectCode::NotFound)
    );
}

#[test]
fn c3_no_key_set() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    st.enclave_pubkey = None;
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(T0, "orch"),
            did,
            FailReason::OtherEnclaveError,
            vec![],
            [0u8; 32],
            sha256(b"pk_C"),
            vec![0x01]
        ),
        Err(RejectCode::NoKeySet)
    );
}

#[test]
fn c3_key_mismatch_after_rotation() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk-old"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let pinned = st.pending_disclosures[&did].entries_root_at_create;
    let old_binding = Binding::Fail {
        chain_id: CHAIN,
        contract_address: ADDR,
        disclosure_id: did,
        reason: FailReason::OtherEnclaveError,
        request_blob_hash: sha256(&st.pending_disclosures[&did].encrypted_request_blob),
        fail_reason_ciphertext_hash: sha256(b""),
        snapshot_root: pinned,
        pk_slot: sha256(b"pk_C"),
        enclave_pubkey_hash: sha256(&pad48(b"pk-old")),
    };
    let old_att = v.attest(&old_binding);
    let after = rotate(&mut st, &v, T0, &pad48(b"pk-new"));
    assert_eq!(
        st.fail_disclosure(
            &v,
            &env(after, "orch"),
            did,
            FailReason::OtherEnclaveError,
            vec![],
            pinned,
            sha256(b"pk_C"),
            old_att
        ),
        Err(RejectCode::KeyMismatch)
    );
}

#[test]
fn c3_request_malformed_accepted() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    let pinned = st.pending_disclosures[&did].entries_root_at_create;
    let pk_c_hash = sha256(b"pk_C");
    let att = fail_att(
        &st,
        &v,
        did,
        FailReason::RequestMalformed,
        b"detail",
        pinned,
        pk_c_hash,
    );
    st.fail_disclosure(
        &v,
        &env(T0, "orch"),
        did,
        FailReason::RequestMalformed,
        b"detail".to_vec(),
        pinned,
        pk_c_hash,
        att,
    )
    .unwrap();
    let fd = &st.disclosures[&did];
    assert!(matches!(
        fd.outcome,
        DisclosureOutcome::Failed {
            reason: FailReason::RequestMalformed,
            ..
        }
    ));
    assert_eq!(fd.snapshot_root, pinned);
}

// ---- C4 paths (all missing) ----

#[test]
fn c4_rejects_non_owner() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    assert_eq!(
        st.cancel_pending_disclosure(&env(T0, "bob"), did),
        Err(RejectCode::Unauthorized)
    );
}

#[test]
fn c4_not_found() {
    let (mut st, _v) = setup();
    assert_eq!(
        st.cancel_pending_disclosure(&env(T0, OWNER), 99),
        Err(RejectCode::NotFound)
    );
}

#[test]
fn c4_owner_cancels() {
    let (mut st, v) = setup();
    register(&mut st, &v, T0, &pad48(b"pk"));
    let did = st.create_disclosure(&env(T0, OWNER), ev(b"req")).unwrap();
    assert!(st.pending_disclosures.contains_key(&did));
    st.cancel_pending_disclosure(&env(T0, OWNER), did).unwrap();
    assert!(!st.pending_disclosures.contains_key(&did));
}

// ---- missing C5 paths ----

#[test]
fn c5_not_found() {
    let (mut st, _v) = setup();
    assert_eq!(
        st.prune_disclosure(&env(T0, "anyone"), 99),
        Err(RejectCode::NotFound)
    );
}

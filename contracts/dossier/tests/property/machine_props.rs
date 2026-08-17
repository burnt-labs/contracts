use std::collections::{BTreeMap, BTreeSet};

use dossier_contract::error::RejectCode;
use dossier_contract::machine::mock::MockVerifier;
use dossier_contract::machine::{Binding, DossierState, Env, KeyBinding, KeyPurpose};
use dossier_contract::merkle::{self, sha256};
use dossier_contract::msg::AdmissionRejectReason;
use dossier_contract::state::{
    dossier_derivation_path, sentinel_pk_unrecoverable, sentinel_snapshot_unavailable, Config,
    DisclosureOutcome, FailReason, FinalizedDisclosure, Hash32, Id, ProofType, Provenance,
    KEY_ROTATION_TIMELOCK_SECONDS, RETENTION_WINDOW_SECONDS,
};
use proptest::prelude::*;

const OWNER: &str = "alice";
const STRANGER: &str = "mallory";
const ORCH: &str = "orchestrator";
const ADDR: &str = "xion1dossier";
const CHAIN: &str = "dossier-test-1";
const FLOOR: u64 = 1_000;
const T0: u64 = 10_000_000;
const BOGUS_ID: Id = 9_999;

// ---- op universe ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttMode {
    Good,
    /// Minted over a perturbed binding tuple (wrong ciphertext /
    /// request hash).
    WrongTuple,
    /// Structurally invalid bytes.
    Garbage,
    /// Minted under a bumped vkey epoch (governance rotation).
    WrongEpoch,
}

#[derive(Debug, Clone)]
enum OpKind {
    Register {
        owner: bool,
        fresh: bool,
        bind_ok: bool,
        key_seed: u8,
    },
    Admit {
        owner: bool,
        schema_ok: bool,
        envelope_ok: bool,
        proof_ok: bool,
        seed: u8,
    },
    AcceptAdm {
        sel: u8,
        att: AttMode,
    },
    RejectAdm {
        sel: u8,
        reason: u8,
        att: AttMode,
    },
    CancelAdm {
        sel: u8,
        owner: bool,
    },
    Revoke {
        sel: u8,
        owner: bool,
    },
    CreateDisc {
        owner: bool,
        envelope_ok: bool,
        seed: u8,
    },
    Fulfil {
        sel: u8,
        root_ok: bool,
        att: AttMode,
        seed: u8,
    },
    FailDisc {
        sel: u8,
        reason: u8,
        /// When set, derive sentinel/payload flags from the reason
        /// (the enclave-honest shape); otherwise use the raw flags
        /// (adversarial shapes).
        consistent: bool,
        sentinel_root: bool,
        sentinel_slot: bool,
        empty_payload: bool,
        att: AttMode,
        seed: u8,
    },
    CancelDisc {
        sel: u8,
        owner: bool,
    },
    Prune {
        sel: u8,
    },
    Propose {
        owner: bool,
        identical: bool,
        fresh: bool,
        key_seed: u8,
    },
    FinalizeRot {
        wrong_epoch: bool,
    },
    CancelRot {
        owner: bool,
    },
}

#[derive(Debug, Clone)]
struct Op {
    dt: u64,
    kind: OpKind,
}

fn mostly_true() -> impl Strategy<Value = bool> {
    prop_oneof![5 => Just(true), 1 => Just(false)]
}

fn arb_att() -> impl Strategy<Value = AttMode> {
    prop_oneof![
        4 => Just(AttMode::Good),
        1 => Just(AttMode::WrongTuple),
        1 => Just(AttMode::Garbage),
        1 => Just(AttMode::WrongEpoch),
    ]
}

fn arb_dt() -> impl Strategy<Value = u64> {
    prop_oneof![
        4 => Just(0u64),
        2 => 1u64..3_600,
        2 => Just(FLOOR),
        1 => Just(3_601u64),
        1 => Just(KEY_ROTATION_TIMELOCK_SECONDS),
        1 => Just(RETENTION_WINDOW_SECONDS),
    ]
}

fn arb_kind() -> impl Strategy<Value = OpKind> {
    prop_oneof![
        2 => (mostly_true(), mostly_true(), mostly_true(), any::<u8>())
            .prop_map(|(owner, fresh, bind_ok, key_seed)| OpKind::Register {
                owner, fresh, bind_ok, key_seed
            }),
        3 => (mostly_true(), mostly_true(), mostly_true(), mostly_true(), any::<u8>())
            .prop_map(|(owner, schema_ok, envelope_ok, proof_ok, seed)| OpKind::Admit {
                owner, schema_ok, envelope_ok, proof_ok, seed
            }),
        3 => (any::<u8>(), arb_att())
            .prop_map(|(sel, att)| OpKind::AcceptAdm { sel, att }),
        1 => (any::<u8>(), any::<u8>(), arb_att())
            .prop_map(|(sel, reason, att)| OpKind::RejectAdm { sel, reason, att }),
        1 => (any::<u8>(), mostly_true())
            .prop_map(|(sel, owner)| OpKind::CancelAdm { sel, owner }),
        1 => (any::<u8>(), mostly_true())
            .prop_map(|(sel, owner)| OpKind::Revoke { sel, owner }),
        3 => (mostly_true(), mostly_true(), any::<u8>())
            .prop_map(|(owner, envelope_ok, seed)| OpKind::CreateDisc {
                owner, envelope_ok, seed
            }),
        3 => (any::<u8>(), mostly_true(), arb_att(), any::<u8>())
            .prop_map(|(sel, root_ok, att, seed)| OpKind::Fulfil { sel, root_ok, att, seed }),
        2 => (
            any::<u8>(), any::<u8>(), mostly_true(), any::<bool>(), any::<bool>(),
            any::<bool>(), arb_att(), any::<u8>(),
        )
            .prop_map(
                |(sel, reason, consistent, sentinel_root, sentinel_slot, empty_payload, att, seed)| {
                    OpKind::FailDisc {
                        sel, reason, consistent, sentinel_root, sentinel_slot,
                        empty_payload, att, seed,
                    }
                },
            ),
        1 => (any::<u8>(), mostly_true())
            .prop_map(|(sel, owner)| OpKind::CancelDisc { sel, owner }),
        1 => any::<u8>().prop_map(|sel| OpKind::Prune { sel }),
        1 => (mostly_true(), any::<bool>(), mostly_true(), any::<u8>())
            .prop_map(|(owner, identical, fresh, key_seed)| OpKind::Propose {
                owner, identical, fresh, key_seed
            }),
        1 => any::<bool>().prop_map(|wrong_epoch| OpKind::FinalizeRot { wrong_epoch }),
        1 => mostly_true().prop_map(|owner| OpKind::CancelRot { owner }),
    ]
}

fn arb_op() -> impl Strategy<Value = Op> {
    (arb_dt(), arb_kind()).prop_map(|(dt, kind)| Op { dt, kind })
}

// ---- helpers ----

use dossier_protocol::testing::seeded_envelope as ev;

fn prov() -> Provenance {
    Provenance {
        attestor: "attestor-x".into(),
        proof_type: ProofType::ZkTls,
    }
}

fn fail_reason(i: u8) -> FailReason {
    match i % 7 {
        0 => FailReason::SnapshotUnavailable,
        1 => FailReason::PredicateUndefined,
        2 => FailReason::SchemaLookupFailed,
        3 => FailReason::RequestDecryptionFailed,
        4 => FailReason::StaleEnclaveKey,
        5 => FailReason::RequestMalformed,
        _ => FailReason::OtherEnclaveError,
    }
}

fn adm_reason(i: u8) -> AdmissionRejectReason {
    match i % 5 {
        0 => AdmissionRejectReason::InvalidProof,
        1 => AdmissionRejectReason::SchemaMismatch,
        2 => AdmissionRejectReason::DecryptionFailed,
        3 => AdmissionRejectReason::StaleEnclaveKey,
        _ => AdmissionRejectReason::OtherEnclaveError,
    }
}

fn pick<V>(m: &BTreeMap<Id, V>, sel: u8) -> Id {
    let keys: Vec<Id> = m.keys().copied().collect();
    if keys.is_empty() || sel == 0xFF {
        BOGUS_ID
    } else {
        keys[sel as usize % keys.len()]
    }
}

/// Small key space so Propose can collide with the current key.
fn key_bytes(seed: u8) -> Vec<u8> {
    // 48 bytes to satisfy the contract's compressed-G1 length check (F-11);
    // distinct per seed so rotations are genuine key changes.
    vec![0x10 + (seed % 8); 48]
}

// ---- driver with ghost state ----

struct Driver {
    st: DossierState,
    v: MockVerifier,
    now: u64,
    /// sha256 of every key that has ever been current (B5/B2 lineage).
    key_hash_history: BTreeSet<Hash32>,
    /// Copies of finalized disclosures at insertion (B7 immutability).
    finalized_ghost: BTreeMap<Id, FinalizedDisclosure>,
    /// Pruned ids never reappear (ids are never reused).
    pruned: BTreeSet<Id>,
}

impl Driver {
    fn new() -> Self {
        let mut reg = BTreeSet::new();
        reg.insert("schema-a".to_string());
        Driver {
            st: DossierState::new(
                OWNER.into(),
                ADDR.into(),
                Config {
                    snapshot_retention_floor_seconds: FLOOR,
                    schema_registry: reg,
                    vkey_name: "zkdcap-v1".into(),
                    min_tcb_eval_num: 0,
                },
            ),
            v: MockVerifier { epoch: 1 },
            now: T0,
            key_hash_history: BTreeSet::new(),
            finalized_ghost: BTreeMap::new(),
            pruned: BTreeSet::new(),
        }
    }

    fn env(&self, sender: &str) -> Env {
        Env {
            now: self.now,
            sender: sender.into(),
            chain_id: CHAIN.into(),
        }
    }

    fn mint_verifier(&self, att: AttMode) -> MockVerifier {
        MockVerifier {
            epoch: if att == AttMode::WrongEpoch {
                self.v.epoch + 1
            } else {
                self.v.epoch
            },
        }
    }

    fn current_pk_hash(&self) -> Hash32 {
        self.st
            .enclave_pubkey
            .as_deref()
            .map(sha256)
            .unwrap_or([0u8; 32])
    }

    fn key_att(&self, pk: &[u8], t: u64, purpose: KeyPurpose, bind_ok: bool) -> Vec<u8> {
        let bound_addr = if bind_ok { ADDR } else { "xion1other" };
        let path = dossier_derivation_path(ADDR);
        self.v.attest_key(
            &KeyBinding {
                pubkey: pk,
                contract_address: bound_addr,
                derivation_path: &path,
                purpose,
            },
            t,
        )
    }

    fn accept_att(&self, id: Id, att: AttMode) -> Vec<u8> {
        if att == AttMode::Garbage {
            return vec![0x01, 0xEE, 0xEE];
        }
        let Some(pa) = self.st.pending_admissions.get(&id) else {
            return vec![0x01; 41];
        };
        let ct_hash = if att == AttMode::WrongTuple {
            sha256(b"ct-OTHER")
        } else {
            sha256(&pa.ciphertext)
        };
        let b = Binding::AdmissionAccept {
            chain_id: CHAIN,
            contract_address: ADDR,
            admission_id: id,
            ciphertext_hash: ct_hash,
            schema_id: pa.schema_id.as_str(),
            enclave_pubkey_hash: self.current_pk_hash(),
        };
        self.mint_verifier(att).attest(&b)
    }

    fn reject_att(&self, id: Id, reason: AdmissionRejectReason, att: AttMode) -> Vec<u8> {
        if att == AttMode::Garbage {
            return vec![0x02, 0xEE];
        }
        let Some(pa) = self.st.pending_admissions.get(&id) else {
            return vec![0x01; 41];
        };
        let ct_hash = if att == AttMode::WrongTuple {
            sha256(b"ct-OTHER")
        } else {
            sha256(&pa.ciphertext)
        };
        let b = Binding::AdmissionReject {
            chain_id: CHAIN,
            contract_address: ADDR,
            admission_id: id,
            reason,
            ciphertext_hash: ct_hash,
            schema_id: pa.schema_id.as_str(),
            enclave_pubkey_hash: self.current_pk_hash(),
        };
        self.mint_verifier(att).attest(&b)
    }

    fn dispatch(&mut self, kind: &OpKind) -> Result<(), RejectCode> {
        match kind {
            OpKind::Register {
                owner,
                fresh,
                bind_ok,
                key_seed,
            } => {
                let pk = key_bytes(*key_seed);
                // The host timestamp no longer gates (freshness moved to the
                // circuit's validity window + tcb_eval floor); `fresh` only
                // varies the embedded value, which no longer changes the outcome.
                let t = if *fresh { self.now } else { self.now - 3_601 };
                let att = self.key_att(&pk, t, KeyPurpose::Register, *bind_ok);
                let sender = if *owner { OWNER } else { STRANGER };
                self.st
                    .register_enclave_key(&self.v, &self.env(sender), pk, att)
            }
            OpKind::Admit {
                owner,
                schema_ok,
                envelope_ok,
                proof_ok,
                seed,
            } => {
                let schema = if *schema_ok { "schema-a" } else { "schema-z" };
                let ct = if *envelope_ok {
                    ev(&[*seed])
                } else {
                    vec![*seed, 1, 2]
                };
                let proof = if *proof_ok { vec![0xAB, *seed] } else { vec![] };
                let sender = if *owner { OWNER } else { STRANGER };
                self.st
                    .admit(&self.env(sender), schema.into(), ct, proof, prov())
                    .map(|_| ())
            }
            OpKind::AcceptAdm { sel, att } => {
                let id = pick(&self.st.pending_admissions, *sel);
                let a = self.accept_att(id, *att);
                self.st.admission_accept(&self.v, &self.env(ORCH), id, a)
            }
            OpKind::RejectAdm { sel, reason, att } => {
                let id = pick(&self.st.pending_admissions, *sel);
                let r = adm_reason(*reason);
                let a = self.reject_att(id, r, *att);
                self.st.admission_reject(&self.v, &self.env(ORCH), id, r, a)
            }
            OpKind::CancelAdm { sel, owner } => {
                let id = pick(&self.st.pending_admissions, *sel);
                let sender = if *owner { OWNER } else { STRANGER };
                self.st.cancel_pending_admission(&self.env(sender), id)
            }
            OpKind::Revoke { sel, owner } => {
                let id = pick(&self.st.entries, *sel);
                let sender = if *owner { OWNER } else { STRANGER };
                self.st.revoke(&self.env(sender), id)
            }
            OpKind::CreateDisc {
                owner,
                envelope_ok,
                seed,
            } => {
                let blob = if *envelope_ok {
                    ev(&[0xC0, *seed])
                } else {
                    vec![*seed]
                };
                let sender = if *owner { OWNER } else { STRANGER };
                self.st
                    .create_disclosure(&self.env(sender), blob)
                    .map(|_| ())
            }
            OpKind::Fulfil {
                sel,
                root_ok,
                att,
                seed,
            } => {
                let id = pick(&self.st.pending_disclosures, *sel);
                let disclosure_ct = vec![0xD0, *seed];
                let pk_c_hash = sha256(b"pk_C");
                let snapshot_root = match self.st.pending_disclosures.get(&id) {
                    Some(pd) if *root_ok => pd.entries_root_at_create,
                    _ => [9u8; 32],
                };
                let attestation = if *att == AttMode::Garbage {
                    vec![0x01, 0xEE]
                } else {
                    match self.st.pending_disclosures.get(&id) {
                        None => vec![0x01; 41],
                        Some(pd) => {
                            let req_hash = if *att == AttMode::WrongTuple {
                                sha256(b"blob-OTHER")
                            } else {
                                sha256(&pd.encrypted_request_blob)
                            };
                            let b = Binding::Fulfil {
                                chain_id: CHAIN,
                                contract_address: ADDR,
                                disclosure_id: id,
                                request_blob_hash: req_hash,
                                disclosure_ciphertext_hash: sha256(&disclosure_ct),
                                snapshot_root,
                                pk_c_hash,
                                enclave_pubkey_hash: self.current_pk_hash(),
                            };
                            self.mint_verifier(*att).attest(&b)
                        }
                    }
                };
                self.st.fulfil_disclosure(
                    &self.v,
                    &self.env(ORCH),
                    id,
                    disclosure_ct,
                    snapshot_root,
                    pk_c_hash,
                    attestation,
                )
            }
            OpKind::FailDisc {
                sel,
                reason,
                consistent,
                sentinel_root,
                sentinel_slot,
                empty_payload,
                att,
                seed,
            } => {
                let id = pick(&self.st.pending_disclosures, *sel);
                let r = fail_reason(*reason);
                let (s_root, s_slot, empty) = if *consistent {
                    (
                        r == FailReason::SnapshotUnavailable,
                        r == FailReason::RequestDecryptionFailed,
                        r == FailReason::RequestDecryptionFailed,
                    )
                } else {
                    (*sentinel_root, *sentinel_slot, *empty_payload)
                };
                let snapshot_root = if s_root {
                    sentinel_snapshot_unavailable()
                } else {
                    self.st
                        .pending_disclosures
                        .get(&id)
                        .map(|pd| pd.entries_root_at_create)
                        .unwrap_or([9u8; 32])
                };
                let pk_slot = if s_slot {
                    sentinel_pk_unrecoverable()
                } else {
                    sha256(b"pk_C")
                };
                let payload: Vec<u8> = if empty { vec![] } else { vec![0xFA, *seed] };
                let attestation = if *att == AttMode::Garbage {
                    vec![0x01, 0xEE]
                } else {
                    match self.st.pending_disclosures.get(&id) {
                        None => vec![0x01; 41],
                        Some(pd) => {
                            let req_hash = if *att == AttMode::WrongTuple {
                                sha256(b"blob-OTHER")
                            } else {
                                sha256(&pd.encrypted_request_blob)
                            };
                            let b = Binding::Fail {
                                chain_id: CHAIN,
                                contract_address: ADDR,
                                disclosure_id: id,
                                reason: r,
                                request_blob_hash: req_hash,
                                fail_reason_ciphertext_hash: sha256(&payload),
                                snapshot_root,
                                pk_slot,
                                enclave_pubkey_hash: self.current_pk_hash(),
                            };
                            self.mint_verifier(*att).attest(&b)
                        }
                    }
                };
                self.st.fail_disclosure(
                    &self.v,
                    &self.env(ORCH),
                    id,
                    r,
                    payload,
                    snapshot_root,
                    pk_slot,
                    attestation,
                )
            }
            OpKind::CancelDisc { sel, owner } => {
                let id = pick(&self.st.pending_disclosures, *sel);
                let sender = if *owner { OWNER } else { STRANGER };
                self.st.cancel_pending_disclosure(&self.env(sender), id)
            }
            OpKind::Prune { sel } => {
                let id = pick(&self.st.disclosures, *sel);
                self.st.prune_disclosure(&self.env("anyone"), id)
            }
            OpKind::Propose {
                owner,
                identical,
                fresh,
                key_seed,
            } => {
                let pk = if *identical {
                    self.st
                        .enclave_pubkey
                        .clone()
                        .unwrap_or_else(|| key_bytes(*key_seed))
                } else {
                    key_bytes(*key_seed)
                };
                // The host timestamp no longer gates (freshness moved to the
                // circuit's validity window + tcb_eval floor); `fresh` only
                // varies the embedded value, which no longer changes the outcome.
                let t = if *fresh { self.now } else { self.now - 3_601 };
                let att = self.key_att(&pk, t, KeyPurpose::Rotate, true);
                let sender = if *owner { OWNER } else { STRANGER };
                self.st
                    .propose_key_rotation(&self.v, &self.env(sender), pk, att)
            }
            OpKind::FinalizeRot { wrong_epoch } => {
                if *wrong_epoch {
                    let v2 = MockVerifier {
                        epoch: self.v.epoch + 1,
                    };
                    self.st.finalize_key_rotation(&v2, &self.env("anyone"))
                } else {
                    self.st.finalize_key_rotation(&self.v, &self.env("anyone"))
                }
            }
            OpKind::CancelRot { owner } => {
                let sender = if *owner { OWNER } else { STRANGER };
                self.st.cancel_key_rotation(&self.env(sender))
            }
        }
    }

    /// Some(owner_flag) for ops whose FIRST guard is require_owner,
    /// so a non-owner sender must reject with Unauthorized exactly.
    /// (Admit is permissionless, so its `owner` flag only varies the sender.)
    fn owner_flag(kind: &OpKind) -> Option<bool> {
        match kind {
            OpKind::Register { owner, .. }
            | OpKind::CancelAdm { owner, .. }
            | OpKind::Revoke { owner, .. }
            | OpKind::CreateDisc { owner, .. }
            | OpKind::CancelDisc { owner, .. }
            | OpKind::Propose { owner, .. }
            | OpKind::CancelRot { owner } => Some(*owner),
            _ => None,
        }
    }

    fn apply(&mut self, op: &Op) {
        self.now += op.dt;
        let pre = self.st.clone();
        let res = self.dispatch(&op.kind);

        // The most valuable property: a rejected op leaves state
        // byte-identical (no partial mutation on any reject path).
        if let Err(code) = res {
            assert_eq!(
                self.st, pre,
                "rejected op mutated state: {op:?} -> {code:?}"
            );
        }

        // Targeted reject oracles.
        if Self::owner_flag(&op.kind) == Some(false) {
            assert_eq!(
                res,
                Err(RejectCode::Unauthorized),
                "non-owner not rejected: {op:?}"
            );
        }
        // (The old "stale D1 -> AttestationNotFresh" assertion is gone: the host
        // timestamp no longer gates; recency is the circuit window + tcb_eval
        // floor, enforced in the verifier, not modeled by the MockVerifier.)
        if let OpKind::Propose {
            owner: true,
            identical: true,
            ..
        } = op.kind
        {
            if pre.enclave_pubkey.is_some() && pre.pending_enclave_key.is_none() {
                assert_eq!(
                    res,
                    Err(RejectCode::IdenticalKey),
                    "identical-key D2 accepted"
                );
            }
        }
        if let OpKind::FinalizeRot { wrong_epoch: false } = op.kind {
            if let Some(p) = pre.pending_enclave_key.as_ref() {
                if self.now < p.unlock_time {
                    assert_eq!(
                        res,
                        Err(RejectCode::TimelockNotElapsed),
                        "premature D3 accepted"
                    );
                }
            }
        }

        // Ghost updates on success.
        if res.is_ok() {
            assert!(self.st.next_id >= pre.next_id, "next_id went backwards");
            if self.st.enclave_pubkey != pre.enclave_pubkey {
                if let Some(pk) = self.st.enclave_pubkey.as_deref() {
                    self.key_hash_history.insert(sha256(pk));
                }
            }
            for (id, e) in &self.st.entries {
                if !pre.entries.contains_key(id) {
                    // B5/B2: new entries are stamped with the hash of
                    // the key current at admission (the post-op key:
                    // A2 cannot change keys).
                    let cur = self
                        .st
                        .enclave_pubkey
                        .as_deref()
                        .expect("entry admitted without a key");
                    assert_eq!(
                        e.enclave_pubkey_hash_at_admission,
                        sha256(cur),
                        "entry stamped with a non-current key hash"
                    );
                }
            }
            for (id, fd) in &self.st.disclosures {
                if !pre.disclosures.contains_key(id) {
                    // B6/B9: finalization consumes a pending.
                    assert!(
                        pre.pending_disclosures.contains_key(id),
                        "finalized disclosure without a pending"
                    );
                    self.finalized_ghost.insert(*id, fd.clone());
                }
            }
            for id in pre.disclosures.keys() {
                if !self.st.disclosures.contains_key(id) {
                    self.pruned.insert(*id);
                }
            }
        }

        self.check_invariants();
    }

    fn check_invariants(&self) {
        let st = &self.st;
        // S1: every allocated id is below the counter.
        for k in st
            .entries
            .keys()
            .chain(st.pending_admissions.keys())
            .chain(st.pending_disclosures.keys())
            .chain(st.disclosures.keys())
        {
            assert!(*k < st.next_id, "map key {k} >= next_id {}", st.next_id);
        }
        // S7: entries_root never lags entries.
        assert_eq!(
            st.entries_root,
            merkle::entries_root_of_map(&st.entries),
            "entries_root lags entries"
        );
        // S6: rotation requires an existing key; unlock_time fixed.
        if let Some(p) = st.pending_enclave_key.as_ref() {
            assert!(
                st.enclave_pubkey.is_some(),
                "S6: pending rotation without a key"
            );
            assert_eq!(
                p.unlock_time,
                p.proposed_at + KEY_ROTATION_TIMELOCK_SECONDS,
                "S6: unlock_time != proposed_at + timelock"
            );
        }
        // B5/B2 lineage: every stamped key hash was once current.
        for e in st.entries.values() {
            assert!(
                self.key_hash_history
                    .contains(&e.enclave_pubkey_hash_at_admission),
                "entry key hash never current"
            );
        }
        // B7: finalized disclosures never mutate; pruned never return.
        for (id, ghost) in &self.finalized_ghost {
            match st.disclosures.get(id) {
                Some(fd) => assert_eq!(fd, ghost, "finalized disclosure {id} mutated"),
                None => assert!(self.pruned.contains(id), "disclosure {id} vanished"),
            }
        }
        for id in &self.pruned {
            assert!(
                !st.disclosures.contains_key(id),
                "pruned disclosure {id} reappeared"
            );
        }
        // B18: sentinel root iff failed-with-snapshot-unavailable.
        for (id, fd) in &st.disclosures {
            let is_sentinel = fd.snapshot_root == sentinel_snapshot_unavailable();
            let is_su = matches!(
                fd.outcome,
                DisclosureOutcome::Failed {
                    reason: FailReason::SnapshotUnavailable,
                    ..
                }
            );
            assert_eq!(is_sentinel, is_su, "B18 violated on disclosure {id}");
        }
    }
}

proptest! {
    #[test]
    fn op_sequences_preserve_invariants(
        ops in proptest::collection::vec(arb_op(), 0..30)
    ) {
        let mut d = Driver::new();
        for op in &ops {
            d.apply(op);
        }
    }
}

/// Deterministic plumbing check: the driver's Good-attestation paths
/// actually succeed, so the sequence property is not vacuously
/// exploring reject paths only.
#[test]
fn driver_good_paths_succeed() {
    let mut d = Driver::new();
    let ops = [
        Op {
            dt: 0,
            kind: OpKind::Register {
                owner: true,
                fresh: true,
                bind_ok: true,
                key_seed: 0,
            },
        },
        Op {
            dt: 0,
            kind: OpKind::Admit {
                owner: true,
                schema_ok: true,
                envelope_ok: true,
                proof_ok: true,
                seed: 1,
            },
        },
        Op {
            dt: 0,
            kind: OpKind::AcceptAdm {
                sel: 0,
                att: AttMode::Good,
            },
        },
        Op {
            dt: 0,
            kind: OpKind::CreateDisc {
                owner: true,
                envelope_ok: true,
                seed: 2,
            },
        },
        Op {
            dt: 0,
            kind: OpKind::Fulfil {
                sel: 0,
                root_ok: true,
                att: AttMode::Good,
                seed: 3,
            },
        },
        Op {
            dt: 0,
            kind: OpKind::CreateDisc {
                owner: true,
                envelope_ok: true,
                seed: 4,
            },
        },
        // SnapshotUnavailable C3: floor elapsed, sentinel root.
        Op {
            dt: FLOOR,
            kind: OpKind::FailDisc {
                sel: 0,
                reason: 0,
                consistent: true,
                sentinel_root: false,
                sentinel_slot: false,
                empty_payload: false,
                att: AttMode::Good,
                seed: 5,
            },
        },
        Op {
            dt: 0,
            kind: OpKind::Propose {
                owner: true,
                identical: false,
                fresh: true,
                key_seed: 1,
            },
        },
        Op {
            dt: KEY_ROTATION_TIMELOCK_SECONDS,
            kind: OpKind::FinalizeRot { wrong_epoch: false },
        },
    ];
    for op in &ops {
        d.apply(op);
    }
    assert_eq!(d.st.entries.len(), 1);
    assert_eq!(d.st.disclosures.len(), 2);
    assert!(d.st.pending_admissions.is_empty());
    assert!(d.st.pending_disclosures.is_empty());
    assert!(d.st.pending_enclave_key.is_none());
    assert_eq!(d.st.enclave_pubkey.as_deref(), Some(&key_bytes(1)[..]));
    assert_eq!(d.key_hash_history.len(), 2);
    assert_eq!(
        d.st.disclosures
            .values()
            .filter(|fd| fd.snapshot_root == sentinel_snapshot_unavailable())
            .count(),
        1
    );
}

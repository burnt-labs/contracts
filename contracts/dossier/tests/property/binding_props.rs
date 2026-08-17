use dossier_contract::machine::mock::canon;
use dossier_contract::machine::Binding;
use dossier_contract::msg::AdmissionRejectReason;
use dossier_contract::state::FailReason;
use proptest::prelude::*;

/// Generate a fresh `String` and leak it for the `&'static str` lifetime
/// that `Binding` requires. Safe in test code because the leaked strings
/// are never freed (fine for a short-lived test binary).
fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn arb_chain_id() -> impl Strategy<Value = &'static str> {
    "[a-z0-9_-]{1,16}".prop_map(leak)
}

fn arb_contract() -> impl Strategy<Value = &'static str> {
    "[a-z0-9]{1,16}".prop_map(leak)
}

fn arb_schema() -> impl Strategy<Value = &'static str> {
    "[a-z]{1,8}".prop_map(leak)
}

fn arb_id() -> impl Strategy<Value = u64> {
    1u64..(1 << 32)
}

fn arb_hash() -> impl Strategy<Value = [u8; 32]> {
    any::<[u8; 32]>()
}

fn arb_adm_reason() -> impl Strategy<Value = AdmissionRejectReason> {
    prop_oneof![
        Just(AdmissionRejectReason::InvalidProof),
        Just(AdmissionRejectReason::SchemaMismatch),
        Just(AdmissionRejectReason::DecryptionFailed),
        Just(AdmissionRejectReason::StaleEnclaveKey),
        Just(AdmissionRejectReason::OtherEnclaveError),
    ]
}

fn arb_fail_reason() -> impl Strategy<Value = FailReason> {
    prop_oneof![
        Just(FailReason::SnapshotUnavailable),
        Just(FailReason::PredicateUndefined),
        Just(FailReason::SchemaLookupFailed),
        Just(FailReason::RequestDecryptionFailed),
        Just(FailReason::StaleEnclaveKey),
        Just(FailReason::OtherEnclaveError),
        Just(FailReason::RequestMalformed),
    ]
}

fn arb_admission_accept() -> impl Strategy<Value = Binding<'static>> {
    (
        arb_chain_id(),
        arb_contract(),
        arb_id(),
        arb_hash(),
        arb_schema(),
        arb_hash(),
    )
        .prop_map(|(chain_id, contract, id, ct_hash, schema, pk_hash)| {
            Binding::AdmissionAccept {
                chain_id,
                contract_address: contract,
                admission_id: id,
                ciphertext_hash: ct_hash,
                schema_id: schema,
                enclave_pubkey_hash: pk_hash,
            }
        })
}

fn arb_admission_reject() -> impl Strategy<Value = Binding<'static>> {
    (
        arb_chain_id(),
        arb_contract(),
        arb_id(),
        arb_adm_reason(),
        arb_hash(),
        arb_schema(),
        arb_hash(),
    )
        .prop_map(
            |(chain_id, contract, id, reason, ct_hash, schema, pk_hash)| Binding::AdmissionReject {
                chain_id,
                contract_address: contract,
                admission_id: id,
                reason,
                ciphertext_hash: ct_hash,
                schema_id: schema,
                enclave_pubkey_hash: pk_hash,
            },
        )
}

fn arb_fulfil() -> impl Strategy<Value = Binding<'static>> {
    (
        arb_chain_id(),
        arb_contract(),
        arb_id(),
        arb_hash(),
        arb_hash(),
        arb_hash(),
        arb_hash(),
        arb_hash(),
    )
        .prop_map(
            |(chain_id, contract, id, req_hash, dc_hash, snap_root, pk_c_hash, pk_hash)| {
                Binding::Fulfil {
                    chain_id,
                    contract_address: contract,
                    disclosure_id: id,
                    request_blob_hash: req_hash,
                    disclosure_ciphertext_hash: dc_hash,
                    snapshot_root: snap_root,
                    pk_c_hash,
                    enclave_pubkey_hash: pk_hash,
                }
            },
        )
}

fn arb_fail() -> impl Strategy<Value = Binding<'static>> {
    (
        arb_chain_id(),
        arb_contract(),
        arb_id(),
        arb_fail_reason(),
        arb_hash(),
        arb_hash(),
        arb_hash(),
        arb_hash(),
        arb_hash(),
    )
        .prop_map(
            |(chain_id, contract, id, reason, req_hash, fc_hash, snap_root, pk_slot, pk_hash)| {
                Binding::Fail {
                    chain_id,
                    contract_address: contract,
                    disclosure_id: id,
                    reason,
                    request_blob_hash: req_hash,
                    fail_reason_ciphertext_hash: fc_hash,
                    snapshot_root: snap_root,
                    pk_slot,
                    enclave_pubkey_hash: pk_hash,
                }
            },
        )
}

fn arb_binding() -> impl Strategy<Value = Binding<'static>> {
    prop_oneof![
        4 => arb_admission_accept(),
        2 => arb_admission_reject(),
        4 => arb_fulfil(),
        2 => arb_fail(),
    ]
}

proptest! {
    // Variant discriminant: the first byte of canon() is unique per variant.
    #[test]
    fn variant_discriminant_is_unique(
        aa in arb_admission_accept(),
        ar in arb_admission_reject(),
        fu in arb_fulfil(),
        fa in arb_fail(),
    ) {
        let bytes = [canon(&aa), canon(&ar), canon(&fu), canon(&fa)];
        prop_assert_eq!(bytes[0][0], 1);
        prop_assert_eq!(bytes[1][0], 2);
        prop_assert_eq!(bytes[2][0], 3);
        prop_assert_eq!(bytes[3][0], 4);
        prop_assert!(bytes[0][0] != bytes[1][0]);
        prop_assert!(bytes[0][0] != bytes[2][0]);
        prop_assert!(bytes[0][0] != bytes[3][0]);
        prop_assert!(bytes[1][0] != bytes[2][0]);
        prop_assert!(bytes[1][0] != bytes[3][0]);
        prop_assert!(bytes[2][0] != bytes[3][0]);
    }

    // Determinism: the same binding always produces the same canonical bytes.
    #[test]
    fn canon_is_deterministic(b in arb_binding()) {
        let c1 = canon(&b);
        let c2 = canon(&b);
        prop_assert_eq!(c1, c2);
    }

    // Injectivity: different bindings produce different canonical bytes.
    #[test]
    fn canon_is_injective(b1 in arb_binding(), b2 in arb_binding()) {
        let c1 = canon(&b1);
        let c2 = canon(&b2);
        if c1 == c2 {
            prop_assert_eq!(b1, b2);
        }
    }

    // For AdmissionAccept, the mutable fields are admission_id,
    // ciphertext_hash, enclave_pubkey_hash.
    #[test]
    fn admission_accept_mutable_fields_sensitive(
        chain_id in arb_chain_id(),
        contract in arb_contract(),
        schema in arb_schema(),
        id in arb_id(),
        ct in arb_hash(),
        pk in arb_hash(),
    ) {
        let base = Binding::AdmissionAccept {
            chain_id, contract_address: contract, admission_id: id,
            ciphertext_hash: ct, schema_id: schema, enclave_pubkey_hash: pk,
        };
        let c_base = canon(&base);
        let id2 = id.wrapping_add(1);
        prop_assert_ne!(canon(&Binding::AdmissionAccept {
            chain_id, contract_address: contract, admission_id: id2,
            ciphertext_hash: ct, schema_id: schema, enclave_pubkey_hash: pk,
        }), c_base.as_slice());
        let mut ct2 = ct; ct2[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::AdmissionAccept {
            chain_id, contract_address: contract, admission_id: id,
            ciphertext_hash: ct2, schema_id: schema, enclave_pubkey_hash: pk,
        }), c_base.as_slice());
        let mut pk2 = pk; pk2[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::AdmissionAccept {
            chain_id, contract_address: contract, admission_id: id,
            ciphertext_hash: ct, schema_id: schema, enclave_pubkey_hash: pk2,
        }), c_base.as_slice());
    }

    // Fulfil: 6 mutable hash/ID fields.
    #[test]
    fn fulfil_mutable_fields_sensitive(
        chain_id in arb_chain_id(),
        contract in arb_contract(),
        id in arb_id(),
        req in arb_hash(),
        dc in arb_hash(),
        snap in arb_hash(),
        pk_c in arb_hash(),
        pk in arb_hash(),
    ) {
        let base = Binding::Fulfil {
            chain_id, contract_address: contract, disclosure_id: id,
            request_blob_hash: req, disclosure_ciphertext_hash: dc,
            snapshot_root: snap, pk_c_hash: pk_c, enclave_pubkey_hash: pk,
        };
        let c_base = canon(&base);

        let mut v = req; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fulfil {
            chain_id, contract_address: contract, disclosure_id: id,
            request_blob_hash: v, disclosure_ciphertext_hash: dc,
            snapshot_root: snap, pk_c_hash: pk_c, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = dc; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fulfil {
            chain_id, contract_address: contract, disclosure_id: id,
            request_blob_hash: req, disclosure_ciphertext_hash: v,
            snapshot_root: snap, pk_c_hash: pk_c, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = snap; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fulfil {
            chain_id, contract_address: contract, disclosure_id: id,
            request_blob_hash: req, disclosure_ciphertext_hash: dc,
            snapshot_root: v, pk_c_hash: pk_c, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = pk_c; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fulfil {
            chain_id, contract_address: contract, disclosure_id: id,
            request_blob_hash: req, disclosure_ciphertext_hash: dc,
            snapshot_root: snap, pk_c_hash: v, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = pk; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fulfil {
            chain_id, contract_address: contract, disclosure_id: id,
            request_blob_hash: req, disclosure_ciphertext_hash: dc,
            snapshot_root: snap, pk_c_hash: pk_c, enclave_pubkey_hash: v,
        }), c_base.as_slice());
    }

    // Fail: reason + 6 hash/ID fields.
    #[test]
    fn fail_mutable_fields_sensitive(
        chain_id in arb_chain_id(),
        contract in arb_contract(),
        id in arb_id(),
        req in arb_hash(),
        fc in arb_hash(),
        snap in arb_hash(),
        slot in arb_hash(),
        pk in arb_hash(),
    ) {
        let base = Binding::Fail {
            chain_id, contract_address: contract, disclosure_id: id,
            reason: FailReason::PredicateUndefined,
            request_blob_hash: req, fail_reason_ciphertext_hash: fc,
            snapshot_root: snap, pk_slot: slot, enclave_pubkey_hash: pk,
        };
        let c_base = canon(&base);

        // different reason
        prop_assert_ne!(canon(&Binding::Fail {
            chain_id, contract_address: contract, disclosure_id: id,
            reason: FailReason::SchemaLookupFailed,
            request_blob_hash: req, fail_reason_ciphertext_hash: fc,
            snapshot_root: snap, pk_slot: slot, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = req; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fail {
            chain_id, contract_address: contract, disclosure_id: id,
            reason: FailReason::PredicateUndefined,
            request_blob_hash: v, fail_reason_ciphertext_hash: fc,
            snapshot_root: snap, pk_slot: slot, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = fc; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fail {
            chain_id, contract_address: contract, disclosure_id: id,
            reason: FailReason::PredicateUndefined,
            request_blob_hash: req, fail_reason_ciphertext_hash: v,
            snapshot_root: snap, pk_slot: slot, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = snap; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fail {
            chain_id, contract_address: contract, disclosure_id: id,
            reason: FailReason::PredicateUndefined,
            request_blob_hash: req, fail_reason_ciphertext_hash: fc,
            snapshot_root: v, pk_slot: slot, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = slot; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fail {
            chain_id, contract_address: contract, disclosure_id: id,
            reason: FailReason::PredicateUndefined,
            request_blob_hash: req, fail_reason_ciphertext_hash: fc,
            snapshot_root: snap, pk_slot: v, enclave_pubkey_hash: pk,
        }), c_base.as_slice());

        let mut v = pk; v[0] ^= 0xFF;
        prop_assert_ne!(canon(&Binding::Fail {
            chain_id, contract_address: contract, disclosure_id: id,
            reason: FailReason::PredicateUndefined,
            request_blob_hash: req, fail_reason_ciphertext_hash: fc,
            snapshot_root: snap, pk_slot: slot, enclave_pubkey_hash: v,
        }), c_base.as_slice());
    }
}

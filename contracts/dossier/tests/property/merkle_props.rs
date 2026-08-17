use std::collections::BTreeMap;

use dossier_contract::merkle::{empty_entries_root, entries_root, entries_root_of_map};
use dossier_contract::state::{
    sentinel_pk_unrecoverable, sentinel_snapshot_unavailable, Entry, EntryId, ProofType, Provenance,
};
use proptest::prelude::*;

fn arb_proof_type() -> impl Strategy<Value = ProofType> {
    prop_oneof![
        Just(ProofType::ZkTls),
        Just(ProofType::ZkDkim),
        Just(ProofType::InstitutionalSignature),
    ]
}

fn next_proof_type(p: ProofType) -> ProofType {
    match p {
        ProofType::ZkTls => ProofType::ZkDkim,
        ProofType::ZkDkim => ProofType::InstitutionalSignature,
        ProofType::InstitutionalSignature => ProofType::ZkTls,
    }
}

prop_compose! {
    fn arb_entry()(
        schema_id in "[a-z]{1,8}",
        ciphertext in proptest::collection::vec(any::<u8>(), 0..48),
        attestor in "[a-z]{1,8}",
        proof_type in arb_proof_type(),
        admission_ts in any::<u64>(),
        admission_attestation in proptest::collection::vec(any::<u8>(), 0..32),
        pk_hash in any::<[u8; 32]>(),
    ) -> Entry {
        Entry {
            schema_id,
            ciphertext,
            provenance: Provenance { attestor, proof_type },
            admission_ts,
            admission_attestation,
            enclave_pubkey_hash_at_admission: pk_hash,
        }
    }
}

/// Ids bounded below 2^32 so `max + 1` for the id-mutation case never
/// overflows.
fn arb_map(min: usize) -> impl Strategy<Value = BTreeMap<EntryId, Entry>> {
    proptest::collection::btree_map(0u64..(1 << 32), arb_entry(), min..10)
}

proptest! {
    // 2.5: leaves are sorted by entry_id; input order never matters.
    #[test]
    fn root_is_input_order_independent(
        pairs in arb_map(0)
            .prop_map(|m| m.into_iter().collect::<Vec<_>>())
            .prop_shuffle()
    ) {
        let map: BTreeMap<EntryId, Entry> = pairs.iter().cloned().collect();
        prop_assert_eq!(entries_root(&pairs), entries_root_of_map(&map));
    }

    // Every committed Entry field, and the id itself, moves the root
    // (the "two states with the same root are byte-equal" premise).
    #[test]
    fn any_single_field_mutation_changes_root(
        map in arb_map(1),
        idx in any::<prop::sample::Index>(),
        field in 0u8..8,
    ) {
        let baseline = entries_root_of_map(&map);
        let keys: Vec<EntryId> = map.keys().copied().collect();
        let id = keys[idx.index(keys.len())];
        let mut mutated = map.clone();
        if field == 7 {
            // move the entry to a fresh id
            let e = mutated.remove(&id).unwrap();
            let fresh = map.keys().max().unwrap() + 1;
            mutated.insert(fresh, e);
        } else {
            let e = mutated.get_mut(&id).unwrap();
            match field {
                0 => e.schema_id.push('x'),
                1 => e.ciphertext.push(0xFF),
                2 => e.provenance.attestor.push('x'),
                3 => e.provenance.proof_type = next_proof_type(e.provenance.proof_type),
                4 => e.admission_ts ^= 1,
                5 => e.admission_attestation.push(0xAA),
                _ => e.enclave_pubkey_hash_at_admission[0] ^= 0xFF,
            }
        }
        prop_assert_ne!(entries_root_of_map(&mutated), baseline);
    }

    // Revocation is root-visible (block B).
    #[test]
    fn removal_changes_root(map in arb_map(1), idx in any::<prop::sample::Index>()) {
        let baseline = entries_root_of_map(&map);
        let keys: Vec<EntryId> = map.keys().copied().collect();
        let mut smaller = map.clone();
        smaller.remove(&keys[idx.index(keys.len())]);
        prop_assert_ne!(entries_root_of_map(&smaller), baseline);
    }

    // Domain separation: non-empty roots collide with neither the
    // empty root nor the C3 sentinels (B18 sentinel discipline).
    #[test]
    fn root_never_collides_with_empty_or_sentinels(map in arb_map(1)) {
        let root = entries_root_of_map(&map);
        prop_assert_ne!(root, empty_entries_root());
        prop_assert_ne!(root, sentinel_snapshot_unavailable());
        prop_assert_ne!(root, sentinel_pk_unrecoverable());
    }
}

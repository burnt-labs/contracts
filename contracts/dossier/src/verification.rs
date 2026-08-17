#![allow(dead_code)]
#![cfg(feature = "verification")]

use sha2::{Digest, Sha256};

use crate::envelope;
use crate::merkle::{empty_entries_root, entries_root, leaf_hash};
use crate::state::{
    dossier_derivation_path, sentinel_pk_unrecoverable, sentinel_snapshot_unavailable, Entry,
    Hash32, ProofType, Provenance,
};


#[cfg(not(kani))]
mod kani {
    pub fn any<T: Default>() -> T {
        T::default()
    }
    pub fn assume(_: bool) {}
}

fn node_hash_oracle(l: &Hash32, r: &Hash32) -> Hash32 {
    let mut h = Sha256::new();
    h.update([0x01u8]);
    h.update(l);
    h.update(r);
    h.finalize().into()
}

fn entry_from_seed(seed: [u8; 4]) -> Entry {
    Entry {
        schema_id: "schema-a".to_string(),
        ciphertext: seed.to_vec(),
        provenance: Provenance {
            attestor: "attestor-x".to_string(),
            proof_type: ProofType::ZkTls,
        },
        admission_ts: u64::from(seed[0]),
        admission_attestation: vec![seed[1], seed[2]],
        enclave_pubkey_hash_at_admission: [seed[3]; 32],
    }
}


#[cfg_attr(kani, kani::proof)]
pub fn sentinel_domain_separation() {
    let su = sentinel_snapshot_unavailable();
    let pk = sentinel_pk_unrecoverable();
    let empty = empty_entries_root();
    assert!(su != pk);
    assert!(su != empty);
    assert!(pk != empty);
}



#[cfg_attr(kani, kani::proof)]
pub fn merkle_promotion_not_duplication() {
    let s1: [u8; 4] = kani::any();
    let s2: [u8; 4] = kani::any();
    let s3: [u8; 4] = kani::any();
    let (e1, e2, e3) = (
        entry_from_seed(s1),
        entry_from_seed(s2),
        entry_from_seed(s3),
    );
    let (l1, l2, l3) = (leaf_hash(1, &e1), leaf_hash(2, &e2), leaf_hash(3, &e3));

    let root = entries_root(&[(1, e1.clone()), (2, e2), (3, e3)]);
    let promoted = node_hash_oracle(&node_hash_oracle(&l1, &l2), &l3);
    let duplicated = node_hash_oracle(&node_hash_oracle(&l1, &l2), &node_hash_oracle(&l3, &l3));
    assert!(root == promoted);
    assert!(root != duplicated);

    assert!(entries_root(&[(1, e1)]) == l1);

    assert!(l3 != node_hash_oracle(&l1, &l2));

    assert!(root != empty_entries_root());
    assert!(root != sentinel_snapshot_unavailable());
    assert!(root != sentinel_pk_unrecoverable());
}



#[cfg_attr(kani, kani::proof)]
pub fn derivation_path_prefix_and_injectivity() {
    let a: [u8; 4] = kani::any();
    let b: [u8; 4] = kani::any();
    let addr_a: String = a.iter().map(|x| char::from(b'a' + x % 26)).collect();
    let addr_b: String = b.iter().map(|x| char::from(b'a' + x % 26)).collect();

    let pa = dossier_derivation_path(&addr_a);
    assert!(pa.as_bytes().starts_with(b"dossier-v1:"));
    assert!(&pa.as_bytes()[11..] == addr_a.as_bytes());

    let pb = dossier_derivation_path(&addr_b);
    if pa == pb {
        assert!(addr_a == addr_b);
    }
}

#[cfg_attr(kani, kani::proof)]
pub fn envelope_well_formed_boundaries() {
    let head: [u8; 5] = kani::any();
    let mut buf = vec![0u8; envelope::MIN_ENVELOPE_LEN];
    buf[..5].copy_from_slice(&head);
    let expected =
        head[..4] == envelope::MAGIC[..] && head[4] == envelope::SUITE_BLS12381_G1_HEG_AES256GCM;
    assert!(envelope::well_formed(&buf) == expected);

    let mut short = vec![0u8; envelope::MIN_ENVELOPE_LEN - 1];
    short[..4].copy_from_slice(envelope::MAGIC);
    short[4] = envelope::SUITE_BLS12381_G1_HEG_AES256GCM;
    assert!(!envelope::well_formed(&short));

    let framed = envelope::frame(&[0u8; envelope::MIN_KEM_BLOB_LEN]);
    assert!(framed.len() == envelope::MIN_ENVELOPE_LEN);
    assert!(envelope::well_formed(&framed));
}

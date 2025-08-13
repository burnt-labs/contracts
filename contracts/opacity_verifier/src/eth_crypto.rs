//! An Ethereum signature has a total length of 65 parts, consisting of three
//! parts:
//! - r: 32 bytes
//! - s: 32 bytes
//! - v: 1 byte
//!
//! r and s together are known as the recoverable signature. v is known as the
//! recovery id, which can take the value of one of 0, 1, 27, and 28.
//!
//! In order to verify a signature, we attempt to recover the signer's pubkey.
//! If the recovered key matches the signer's address, we consider the signature
//! valid.
//!
//! The address is the last 20 bytes of the hash keccak256(pubkey_bytes).
//!
//! Before a message is signed, it is prefixed with the bytes: b"\x19Ethereum Signed Message:\n".
//!
//! Adapted from
//! - sig verification:
//!   https://github.com/gakonst/ethers-rs/blob/master/ethers-core/src/types/signature.rs
//! - hash:
//!   https://github.com/gakonst/ethers-rs/blob/master/ethers-core/src/utils/hash.rs

use tiny_keccak::{Hasher, Keccak};

use crate::error::{ContractError, ContractResult};

pub fn hash_message(msg: &[u8]) -> [u8; 32] {
    const PREFIX: &str = "\x19Ethereum Signed Message:\n";

    let mut bytes = vec![];
    bytes.extend_from_slice(PREFIX.as_bytes());
    bytes.extend_from_slice(msg.len().to_string().as_bytes());
    bytes.extend_from_slice(msg);

    keccak256(&bytes)
}

pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];

    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut output);

    output
}

pub fn normalize_recovery_id(id: u8) -> ContractResult<u8> {
    match id {
        0 | 1 => Ok(id),
        27 => Ok(0),
        28 => Ok(1),
        _ => Err(ContractError::InvalidRecoveryId),
    }
}

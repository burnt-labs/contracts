use crate::error::ContractResult;
use cosmwasm_std::Binary;
use p256::ecdsa::VerifyingKey;
use p256::elliptic_curve::PublicKey;
use p256::{AffinePoint, NistP256};

pub fn verify(tx_hash: &Vec<u8>, sig_bytes: &[u8], pubkey_bytes: Binary) -> ContractResult<bool> {
    // let verifying_key: VerifyingKey = VerifyingKey::from(&pubkey);
    let pubkey = p256::PublicKey::from_sec1_bytes(&pubkey_bytes)?;

    Ok(true)
}

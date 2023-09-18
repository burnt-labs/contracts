use crate::error::ContractError;
use bech32::{ToBase32, Variant};
use cosmwasm_std::{Addr, Api, ContractResult};
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

pub fn sha256(msg: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(msg);
    hasher.finalize().to_vec()
}

fn ripemd160(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Ripemd160::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

pub const CHAIN_BECH_PREFIX: &str = "xion";
pub fn derive_addr(prefix: &str, pubkey_bytes: &[u8]) -> Result<String, ContractError> {
    let address_bytes = ripemd160(&sha256(&pubkey_bytes));
    let address_str = bech32::encode(prefix, address_bytes.to_base32(), Variant::Bech32);

    return match address_str {
        Ok(s) => Ok(s),
        Err(err) => Err(err.into()),
    };
}

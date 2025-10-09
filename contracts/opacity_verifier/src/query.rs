use cosmwasm_std::{Addr, Api, Order, StdError, StdResult, Storage};
use crate::error::{ContractError, ContractResult};
use crate::state::{ADMIN, VERIFICATION_KEY_ALLOW_LIST};

pub fn verify(api: &dyn Api, store: &dyn Storage, signature: String, message: String) -> ContractResult<bool> {
    // 1. Get the signature and message from the response
    let signature_hex = signature.trim_start_matches("0x");
    let sig_bytes = hex::decode(signature_hex)?;
    if sig_bytes.len() < 65 {
        return Err(ContractError::ShortSignature);
    }

    // 2. Recover the public key
    let msg_hash_bytes = crate::eth_crypto::hash_message(message.as_bytes());
    let recoverable_sig = &sig_bytes[..64];
    let recovery_id = crate::eth_crypto::normalize_recovery_id(sig_bytes[64])?;

    let pk_bytes = api.secp256k1_recover_pubkey(&msg_hash_bytes, recoverable_sig, recovery_id)?;
    let hash = crate::eth_crypto::keccak256(&pk_bytes[1..]);
    let recovered_addr = &hash[12..];
    let recovered_address_lower = hex::encode(recovered_addr);

    // 3. Fetch and check against allowlist
    let key_found = VERIFICATION_KEY_ALLOW_LIST.has(store, recovered_address_lower);
    Ok(key_found)
}

pub fn verify_query(store: &dyn Storage, api: &dyn Api, signature: String, message: String) -> StdResult<bool> {
    match verify(api, store, signature, message) {
        Ok(b) => Ok(b),
        Err(error) => Err(StdError::generic_err(error.to_string())),
    }
}

pub fn verification_keys(store: &dyn Storage) -> StdResult<Vec<String>> {
    Ok(VERIFICATION_KEY_ALLOW_LIST
        .keys(store, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect())
}

pub fn admin(store: &dyn Storage) -> StdResult<Addr> {
    ADMIN.load(store)
}
use alloy_signer::Signature;
use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage};
use crate::error::ContractResult;
use crate::state::{ADMIN, VERIFICATION_KEY_ALLOW_LIST};

pub fn verify(store: &dyn Storage, signature: String, message: String) -> ContractResult<bool> {
    // 1. Get the signature and message from the response
    let signature_hex = signature.trim_start_matches("0x");
    let signature_bytes = hex::decode(signature_hex)?;

    // 2. Recover the public key
    let signature = Signature::try_from(signature_bytes.as_slice())?;
    let recovered_address = signature.recover_address_from_msg(message.as_bytes())?;
    let recovered_address_lower = recovered_address.to_string().to_lowercase();
    
    // 3. Fetch and check against allowlist
    let key_found = VERIFICATION_KEY_ALLOW_LIST.has(store, recovered_address_lower);
    Ok(key_found)
}

pub fn verify_query(store: &dyn Storage, signature: String, message: String) -> StdResult<bool> {
    match verify(store, signature, message) {
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
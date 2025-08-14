use std::io::Cursor;
use appattest_rs::attestation::Attestation;
use cosmwasm_std::{Binary, StdError};
use crate::error::ContractResult;
use ciborium::from_reader;


pub fn verify_attestation(app_id: String, key_id: String, challenge: Binary, cbor_data: Binary, timestamp: i64, dev_env: Option<bool>) -> ContractResult<()> {
    let cursor = Cursor::new(cbor_data);
    let attestation_result: Result<Attestation, _> = from_reader(cursor);
    let attestation = match attestation_result {
        Ok(attestation) => attestation,
        Err(err) => return Err(StdError::generic_err(err.to_string()).into()),
    };

    match attestation.verify(challenge.to_base64().as_str(), app_id.as_str(), key_id.as_str(), timestamp, dev_env) {
        Ok(_) => Ok(()),
        Err(err) => Err(StdError::generic_err(err.to_string()).into()),
    }
}

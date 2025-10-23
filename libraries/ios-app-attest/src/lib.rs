mod error;
pub mod msg;

use std::io::Cursor;
use appattest_rs::attestation::Attestation;
use ciborium::from_reader;
use cosmwasm_std::{Binary, StdError};

// // the random function must be disabled in cosmwasm
// #[cfg(not(feature = "library"))]
// use core::num::NonZeroU32;
//
// #[cfg(not(feature = "library"))]
// use getrandom::Error;
// #[cfg(not(feature = "library"))]
// pub fn always_fail(_buf: &mut [u8]) -> Result<(), Error> {
//     let code = NonZeroU32::new(Error::CUSTOM_START).unwrap();
//     Err(Error::from(code))
// }
// #[cfg(not(feature = "library"))]
// use getrandom::register_custom_getrandom;
//
// #[cfg(not(feature = "library"))]
// register_custom_getrandom!(always_fail);


pub fn verify_attestation(app_id: String, key_id: String, challenge: Binary, cbor_data: Binary, timestamp: i64, dev_env: Option<bool>) -> Result<(), StdError> {
    let cursor = Cursor::new(cbor_data);
    let attestation_result: Result<Attestation, _> = from_reader(cursor);
    let attestation = match attestation_result {
        Ok(attestation) => attestation,
        Err(err) => return Err(StdError::generic_err(err.to_string())),
    };

    match attestation.verify(challenge.to_base64().as_str(), app_id.as_str(), key_id.as_str(), timestamp, dev_env) {
        Ok(_) => Ok(()),
        Err(err) => Err(StdError::generic_err(err.to_string())),
    }
}
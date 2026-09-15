pub mod contract;
mod error;
mod eth_crypto;
pub mod msg;
mod query;
mod state;

pub const CONTRACT_NAME: &str = "opacity_verifier";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// the random function must be disabled in cosmwasm
use core::num::NonZeroU32;
use getrandom::Error;

pub fn always_fail(_buf: &mut [u8]) -> Result<(), Error> {
    let code = NonZeroU32::new(Error::CUSTOM_START).unwrap();
    Err(Error::from(code))
}
use getrandom::register_custom_getrandom;
register_custom_getrandom!(always_fail);

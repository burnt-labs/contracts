extern crate core;

mod auth;
pub mod contract;
pub mod error;
pub mod execute;
pub mod msg;
pub mod proto;
pub mod query;
pub mod state;

pub const CONTRACT_NAME: &str = "account";
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

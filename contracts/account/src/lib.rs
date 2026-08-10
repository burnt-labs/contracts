extern crate core;

mod auth;
pub mod contract;
pub mod error;
pub mod execute;
pub mod msg;
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

/// Extra exports to be able to test the xion library externally
pub mod testing {
    pub use super::auth::testing::wrap_message;
    pub use super::auth::util;
}

pub use auth::AddAuthenticator;

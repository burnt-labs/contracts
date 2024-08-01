extern crate core;

#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
mod execute;
mod msg;
mod state;

mod grant;
mod query;

mod unit_test;

pub const CONTRACT_NAME: &str = "treasury";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

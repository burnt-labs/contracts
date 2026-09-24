extern crate core;

pub mod contract;
mod error;
pub mod msg;
mod state;

pub const CONTRACT_NAME: &str = "user-map";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

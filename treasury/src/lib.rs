extern crate core;

mod contract;
mod error;
mod execute;
mod msg;
mod proto;
mod state;

mod grant;
mod query;

pub const CONTRACT_NAME: &str = "treasury";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

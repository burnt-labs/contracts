#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
mod execute;
pub mod msg;
mod state;

pub use crate::error::ContractError;

pub const CONTRACT_NAME: &str = "cw721-proxy";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

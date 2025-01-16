#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
mod execute;
mod msg;
mod state;

pub const CONTRACT_NAME: &str = "proxy";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

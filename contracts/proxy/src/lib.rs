#[cfg(not(feature = "library"))]
pub mod contract;
mod msg;
mod state;
mod execute;
mod error;

pub const CONTRACT_NAME: &str = "proxy";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

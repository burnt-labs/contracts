pub mod contract;
pub mod msg;
mod query;
mod state;
mod error;

pub const CONTRACT_NAME: &str = "opacity_verifier";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

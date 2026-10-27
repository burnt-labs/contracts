pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

pub const CONTRACT_NAME: &str = "one-way-swap";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

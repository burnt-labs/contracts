pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod plugin;

pub const CONTRACT_NAME: &str = "asset";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
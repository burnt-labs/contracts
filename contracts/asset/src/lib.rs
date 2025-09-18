pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod plugin;
pub mod traits;
pub mod execute;

pub const CONTRACT_NAME: &str = "asset";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
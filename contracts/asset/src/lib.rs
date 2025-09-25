pub mod contract;
pub mod error;
pub mod execute;
pub mod msg;
pub mod plugin;
pub mod state;
pub mod traits;

pub const CONTRACT_NAME: &str = "asset";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

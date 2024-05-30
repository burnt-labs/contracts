mod msg;
mod contract;
mod grant;
mod state;
mod error;
mod execute;

pub const CONTRACT_NAME: &str = "treasury";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

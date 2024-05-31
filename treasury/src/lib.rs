mod contract;
mod error;
mod execute;
mod grant;
mod msg;
mod proto;
mod state;

pub const CONTRACT_NAME: &str = "treasury";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

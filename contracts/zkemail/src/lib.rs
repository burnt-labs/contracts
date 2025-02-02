pub mod ark_verifier;
pub mod commit;
pub mod contract;
mod error;
mod groth16;
pub mod msg;
mod state;

pub const CONTRACT_NAME: &str = "zkemail-verifier";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

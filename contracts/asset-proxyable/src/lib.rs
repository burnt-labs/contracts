//! `asset-proxyable`: the XION asset contract plus a narrow, typed envelope that lets an
//! explicitly trusted proxy contract execute a closed set of user actions (`Burn`,
//! `ApproveAll`, `RevokeAll`) on behalf of the real sender.
//!
//! Everything that is not the envelope or trusted-proxy management is delegated verbatim
//! to the base `asset` contract, so collections deployed from the base code id have no
//! proxy surface at all and this crate stays a thin wrapper.

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

pub const CONTRACT_NAME: &str = "asset-proxyable";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

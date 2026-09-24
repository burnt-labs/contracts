//! `xion-asset-proxy`: a fixed-address contract that Treasury authz/feegrant configs can
//! name once, and that relays a closed set of typed user actions to allowlisted
//! `asset-proxyable` collections. The effective sender is always the proxy's own
//! `info.sender`; the proxy never accepts a sender parameter and never holds funds.
//!
//! Two user entrypoints, `SponsoredBurn` and `SponsoredApproval`, exist only so an authz
//! grant can budget the irreversible action separately from the cheap one.

pub mod contract;
pub mod error;
pub mod msg;
pub mod policy;
pub mod state;

#[cfg(test)]
mod tests;

pub const CONTRACT_NAME: &str = "xion-asset-proxy";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

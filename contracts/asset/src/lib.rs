// #[cfg(all(feature = "asset_base", feature = "crossmint"))]
// compile_error!("Features `asset_base` and `crossmint` cannot be enabled at the same time. Pick one.");

pub mod contracts;
pub mod default_plugins;
pub mod error;
pub mod execute;
pub mod msg;
pub mod plugin;
pub mod state;
#[cfg(test)]
mod tests;
pub mod traits;

pub const CONTRACT_NAME: &str = "asset";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

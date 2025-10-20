// #[cfg(all(feature = "asset_base", feature = "crossmint"))]
// compile_error!("Features `asset_base` and `crossmint` cannot be enabled at the same time. Pick one.");

pub mod contracts;
pub mod error;
pub mod execute;
pub mod msg;
pub mod plugin;
pub mod state;
pub mod traits;
pub mod default_plugins;
mod test;

pub const CONTRACT_NAME: &str = "asset";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

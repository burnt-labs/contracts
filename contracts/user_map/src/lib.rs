extern crate core;

#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
pub mod msg;
mod state;

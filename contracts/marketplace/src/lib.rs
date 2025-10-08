extern crate core;

#[cfg(not(feature = "library"))]
pub mod contract;
pub mod error;
pub mod events;
pub mod helpers;
pub mod msg;
pub mod query;
pub mod state;

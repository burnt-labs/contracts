extern crate core;

#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
mod events;
mod helpers;
pub mod msg;
mod state;

pub use dossier_protocol::quote::*;

#[cfg(not(feature = "mock-attestation"))]
pub mod xion;

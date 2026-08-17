//! Property layer (verification pyramid, between unit tests and Kani).
//!
//! merkle_props runs under plain `cargo test`. machine_props drives the
//! state machine through arbitrary op sequences with the mock verifier
//! and needs `cargo test -p dossier-contract --features mock-attestation`
//! (machine::mock is cfg-gated, same arrangement as tests/shell.rs).

#[path = "property/merkle_props.rs"]
mod merkle_props;

#[cfg(feature = "mock-attestation")]
#[path = "property/machine_props.rs"]
mod machine_props;

#[cfg(feature = "mock-attestation")]
#[path = "property/binding_props.rs"]
mod binding_props;

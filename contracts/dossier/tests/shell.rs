#![cfg(feature = "mock-attestation")]

#[path = "shell/admission.rs"]
mod admission;
#[path = "shell/common.rs"]
mod common;
#[path = "shell/disclosure.rs"]
mod disclosure;
#[path = "shell/key_rotation.rs"]
mod key_rotation;
#[path = "shell/queries.rs"]
mod queries;
#[path = "shell/real_key.rs"]
mod real_key;
#[path = "shell/validation.rs"]
mod validation;

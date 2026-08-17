#![forbid(unsafe_code)]

#[cfg(all(
    target_arch = "wasm32",
    any(feature = "mock-attestation", feature = "accepting-proof"),
    not(feature = "unsafe-devnet")
))]
compile_error!(
    "mock-attestation / accepting-proof bypass attestation and are devnet-only; \
     building them into a wasm contract requires `--features unsafe-devnet`"
);

pub use dossier_protocol::{envelope, error, merkle, msg, state};

pub mod contract;
pub mod machine;
pub mod quote;
#[cfg(feature = "verification")]
pub mod verification;

pub use error::RejectCode;
pub use merkle::{empty_entries_root, entries_root, leaf_hash};
pub use state::{
    dossier_derivation_path, sentinel_pk_unrecoverable, sentinel_snapshot_unavailable,
};

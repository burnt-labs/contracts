#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Verification(#[from] cosmwasm_std::VerificationError),

    #[error(transparent)]
    RecoverPubkey(#[from] cosmwasm_std::RecoverPubkeyError),

    #[error(transparent)]
    FromHex(#[from] hex::FromHexError),

    #[error("signature is invalid")]
    InvalidSignature,

    #[error("signature is empty")]
    EmptySignature,

    #[error("short signature")]
    ShortSignature,

    #[error("only the contract itself can call this method")]
    Unauthorized,

    #[error("recovery id can only be one of 0, 1, 27, 28")]
    InvalidRecoveryId,

    #[error("the pubkey recovered from the signature does not match")]
    RecoveredPubkeyMismatch,

    #[error("cannot delete the last authenticator")]
    MinimumAuthenticatorCount,
}

pub type ContractResult<T> = Result<T, ContractError>;

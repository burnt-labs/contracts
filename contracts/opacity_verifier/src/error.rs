#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    HexError(#[from] hex::FromHexError),

    #[error(transparent)]
    UTF8Error(#[from] std::str::Utf8Error),

    #[error(transparent)]
    RecoverPubkeyError(#[from] cosmwasm_std::RecoverPubkeyError),

    // #[error(transparent)]
    // AlloySignatureError(#[from] alloy_primitives::SignatureError),

    #[error("only the admin can call this method")]
    Unauthorized,

    #[error("short signature")]
    ShortSignature,

    #[error("recovery id can only be one of 0, 1, 27, 28")]
    InvalidRecoveryId,
}

pub type ContractResult<T> = Result<T, ContractError>;

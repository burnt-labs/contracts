#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Verification(#[from] cosmwasm_std::VerificationError),

    #[error(transparent)]
    RecoverPubkey(#[from] cosmwasm_std::RecoverPubkeyError),

    #[error(transparent)]
    FromHex(#[from] hex::FromHexError),

    #[error(transparent)]
    Bech32(#[from] bech32::Error),

    #[error(transparent)]
    UTF8Error(#[from] std::str::Utf8Error),

    #[error(transparent)]
    Base64Decode(#[from] base64::DecodeError),

    #[error(transparent)]
    Rsa(#[from] rsa::Error),

    #[error(transparent)]
    P256EllipticCurve(#[from] p256::elliptic_curve::Error),

    #[error(transparent)]
    P256EcdsaCurve(#[from] p256::ecdsa::Error),

    #[error("error rebuilding key")]
    RebuildingKey,

    #[error("signature is invalid")]
    InvalidSignature,

    #[error("signature is invalid. expected: {expected}, received {received}")]
    InvalidSignatureDetail { expected: String, received: String },

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

    #[error("invalid time on signature. current: {current} received: {received}")]
    InvalidTime { current: u64, received: u64 },

    #[error("invalid jwt aud")]
    InvalidJWTAud,

    #[error("invalid token")]
    InvalidToken,

    #[error("url parse error: {url}")]
    URLParse { url: String },

    #[error("cannot override existing authenticator at index {index}")]
    OverridingIndex { index: u8 },
}

pub type ContractResult<T> = Result<T, ContractError>;

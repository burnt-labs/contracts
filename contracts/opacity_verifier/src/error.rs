#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    HexError(#[from] hex::FromHexError),

    #[error(transparent)]
    AlloySignatureError(#[from] alloy_primitives::SignatureError),

    #[error("only the admin can call this method")]
    Unauthorized,
}

pub type ContractResult<T> = Result<T, ContractError>;

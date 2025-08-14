#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    
    #[error("invalid app id")]
    InvalidAppId,
}

pub type ContractResult<T> = Result<T, ContractError>;

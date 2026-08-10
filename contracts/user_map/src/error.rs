#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error("stored value exceeds maximum length of {0} bytes")]
    ValueTooLong(usize),
}

#[allow(dead_code)]
pub type ContractResult<T> = Result<T, ContractError>;

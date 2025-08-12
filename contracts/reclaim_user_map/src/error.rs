#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    
    #[error("json key missing")]
    JSONKeyMissing,

    #[error("extracted paramters missing")]
    ExtractedParametersMissing,
    
    #[error("claim key invalid")]
    ClaimKeyInvalid,
}

pub type ContractResult<T> = Result<T, ContractError>;

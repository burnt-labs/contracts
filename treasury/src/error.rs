#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Prost(#[from] prost::DecodeError),
}

pub type ContractResult<T> = Result<T, ContractError>;

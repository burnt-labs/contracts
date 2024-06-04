#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Prost(#[from] prost::DecodeError),

    #[error("authz grant not found")]
    AuthzGrantNotFound,

    #[error("authz grant has no authorization")]
    AuthzGrantNoAuthorization,

    #[error("authz grant did not match config")]
    AuthzGrantMistmatch,

    #[error("invalid allowance type: {msg_type_url}")]
    InvalidAllowanceType { msg_type_url: String },

    #[error("allowance unset")]
    AllowanceUnset,

    #[error("config mismatch")]
    ConfigurationMismatch,

    #[error("unauthorized")]
    Unauthorized,
}

pub type ContractResult<T> = Result<T, ContractError>;

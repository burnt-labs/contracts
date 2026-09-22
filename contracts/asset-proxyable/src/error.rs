use cosmwasm_std::StdError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Asset(#[from] asset::error::ContractError),

    #[error("{0}")]
    Cw721(#[from] cw721::error::Cw721ContractError),

    #[error("{0}")]
    Payment(#[from] cw_utils::PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Only the token owner may burn through a proxy: {token_id}")]
    NotTokenOwner { token_id: String },

    #[error("Invalid trusted proxy: {reason}")]
    InvalidTrustedProxy { reason: String },

    #[error("Too many trusted proxies (max {max})")]
    TooManyTrustedProxies { max: usize },

    #[error("Trusted proxy already registered: {proxy}")]
    TrustedProxyAlreadyExists { proxy: String },

    #[error("Trusted proxy not found: {proxy}")]
    TrustedProxyNotFound { proxy: String },

    #[error("Cannot renounce creator ownership while trusted proxies are registered")]
    TrustedProxiesNotEmpty {},

    #[error("Cannot migrate from {contract} {version}")]
    InvalidMigration { contract: String, version: String },
}

pub type ContractResult<T> = Result<T, ContractError>;

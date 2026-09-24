use cosmwasm_std::StdError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Payment(#[from] cw_utils::PaymentError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("The proxy cannot target itself")]
    SelfTarget {},

    #[error("Collection is not allowlisted: {collection}")]
    CollectionNotAllowed { collection: String },

    #[error("Collection is already allowlisted: {collection}")]
    CollectionAlreadyAllowed { collection: String },

    #[error("Collection is quarantined (revocation only): {collection}")]
    CollectionQuarantined { collection: String },

    #[error("Collection is not quarantined: {collection}")]
    CollectionNotQuarantined { collection: String },

    #[error("Too many collections (max {max})")]
    TooManyCollections { max: usize },

    #[error("Operator is not allowed: {operator}")]
    OperatorNotAllowed { operator: String },

    #[error("Operator not found: {operator}")]
    OperatorNotFound { operator: String },

    #[error("Too many operators (max {max})")]
    TooManyOperators { max: usize },

    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },

    #[error("Approval expiry must be a timestamp within {max_seconds} seconds")]
    ApprovalExpiryOutOfBounds { max_seconds: u64 },
}

pub type ContractResult<T> = Result<T, ContractError>;

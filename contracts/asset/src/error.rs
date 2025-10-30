#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ContractError {
    // Generic errors
    #[error("{0}")]
    Std(#[from] cosmwasm_std::StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Listing already exists: {id}")]
    ListingAlreadyExists { id: String },

    #[error("Listing not found: {id}")]
    ListingNotFound { id: String },

    #[error("Reserved asset: {id}")]
    ReservedAsset { id: String }, // e.g. listing is reserved

    #[error("Reservation not found: {id}")]
    ReservationNotFound { id: String },

    #[error("Invalid reservation expiration: {reserved_until}")]
    InvalidReservationExpiration { reserved_until: u64 },

    #[error("Invalid listing price: {price}")]
    InvalidListingPrice { price: u128 },

    #[error("Invalid payment: {price} {denom}")]
    InvalidPayment { price: u128, denom: String },

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("No payment")]
    NoPayment {},

    #[error("Multiple payments sent")]
    MultiplePaymentsSent {},

    #[error("Plugin error: {msg}")]
    PluginError { msg: String },

    #[error("Stale listing")]
    StaleListing {},
}

impl From<ContractError> for cw721::error::Cw721ContractError {
    fn from(value: ContractError) -> Self {
        cw721::error::Cw721ContractError::Std(cosmwasm_std::StdError::generic_err(
            value.to_string(),
        ))
    }
}

impl From<cw721::error::Cw721ContractError> for ContractError {
    fn from(value: cw721::error::Cw721ContractError) -> Self {
        ContractError::Std(cosmwasm_std::StdError::generic_err(value.to_string()))
    }
}

impl From<ContractError> for cosmwasm_std::StdError {
    fn from(value: ContractError) -> Self {
        cosmwasm_std::StdError::generic_err(value.to_string())
    }
}

pub type ContractResult<T> = Result<T, ContractError>;

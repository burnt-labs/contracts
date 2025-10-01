use cosmwasm_std::{Coin, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Offer already exists: {id}")]
    OfferAlreadyExists { id: String },

    #[error("Invalid fee rate")]
    InvalidFeeRate {},

    #[error("Invalid price: {expected} != {actual}")]
    InvalidPrice { expected: Coin, actual: Coin },
}

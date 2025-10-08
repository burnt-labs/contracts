use cosmwasm_std::{Coin, StdError};
use cw_utils::PaymentError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },

    #[error("Offer already exists: {id}")]
    OfferAlreadyExists { id: String },

    #[error("Invalid fee rate")]
    InvalidFeeRate {},

    #[error("Listing not found: {id}")]
    ListingNotFound { id: String },

    #[error("Not listed")]
    NotListed {},

    #[error("Already listed")]
    AlreadyListed {},

    #[error("Invalid listing denom: expected {expected}, got {actual}")]
    InvalidListingDenom { expected: String, actual: String },

    #[error("Invalid listing status: expected {expected}, got {actual}")]
    InvalidListingStatus { expected: String, actual: String },

    #[error("Invalid price: {expected} != {actual}")]
    InvalidPrice { expected: Coin, actual: Coin },

    #[error("Invalid payment: {expected} != {actual}")]
    InvalidPayment { expected: Coin, actual: Coin },

    #[error("Invalid seller")]
    InvalidSeller {},

    #[error("{0}")]
    PaymentError(#[from] PaymentError),
}

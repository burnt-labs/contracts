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

    #[error("Invalid fee recipient")]
    InvalidFeeRecipient {},

    #[error("Invalid listing status: expected {expected}, got {actual}")]
    InvalidListingStatus { expected: String, actual: String },

    #[error("Invalid price: {expected} != {actual}")]
    InvalidPrice { expected: Coin, actual: Coin },

    #[error("Invalid payment: {expected} != {actual}")]
    InvalidPayment { expected: Coin, actual: Coin },

    #[error("Invalid seller")]
    InvalidSeller {},

    #[error("Invalid token: expected {expected}, got {actual}")]
    InvalidTokenId { expected: String, actual: String },

    #[error("Invalid collection: expected {expected}, got {actual}")]
    InvalidCollection { expected: String, actual: String },

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Insufficient funds")]
    InsuficientFunds {},

    #[error("Offers disabled")]
    OfferesDisabled {},

    #[error("Pending sale already exists: {collection}, {token_id}")]
    PendingSaleAlreadyExists {
        collection: String,
        token_id: String,
    },

    #[error("Pending sale expired: {id}")]
    PendingSaleExpired { id: String },

    #[error("Pending sale not yet expired: {id}")]
    PendingSaleNotExpired { id: String },
}

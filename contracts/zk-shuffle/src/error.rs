use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("card already decrypted by this player")]
    AlreadyDecrypted,

    #[error("callback message missing")]
    MissingCallback,

    #[error("operation not supported")]
    NotSupported,

    #[error("proof verification failed")]
    InvalidProof,
}

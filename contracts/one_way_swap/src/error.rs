use cosmwasm_std::{CheckedMultiplyFractionError, StdError, Uint128};
use cw_utils::PaymentError;

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    Math(#[from] CheckedMultiplyFractionError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("denoms must be non-empty and contain no whitespace")]
    InvalidDenom,

    #[error("in_denom and out_denom must differ")]
    SameDenom,

    #[error("rate must be greater than zero")]
    ZeroRate,

    #[error("swap amount too small: output rounds to zero")]
    SwapTooSmall,

    #[error("output {out} is below min_out {min_out}")]
    MinOutNotMet { out: Uint128, min_out: Uint128 },
}

pub type ContractResult<T> = Result<T, ContractError>;

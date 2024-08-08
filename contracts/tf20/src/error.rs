use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(transparent)]
    CW20Base(#[from] cw20_base::ContractError),

    #[error("only the contract admin can call this method")]
    Unauthorized,
}

pub type ContractResult<T> = Result<T, ContractError>;

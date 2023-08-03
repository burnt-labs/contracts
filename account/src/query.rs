use cosmwasm_std::{Order, StdError, StdResult, Storage};

use crate::state::AUTHENTICATORS;

pub fn authenticator_ids(store: &dyn Storage) -> StdResult<Vec<u8>> {
    Ok(AUTHENTICATORS
        .keys(store, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect())
}

pub fn authenticator_by_id(store: &dyn Storage, id: u8) -> StdResult<String> {
    let auth = AUTHENTICATORS.load(store, id)?;

    match cosmwasm_std::to_binary(&auth) {
        Ok(auth_bz) => Ok(auth_bz.to_string()),
        Err(error) => Err(StdError::GenericErr {msg: error.to_string()}),
    }
}

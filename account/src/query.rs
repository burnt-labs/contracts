use cosmwasm_std::{Order, StdError, StdResult, Storage, Uint64};

use crate::state::AUTHENTICATORS;

pub fn authenticator_ids(store: &dyn Storage) -> StdResult<Vec<Uint64>> {
    Ok(AUTHENTICATORS
        .keys(store, None, None, Order::Ascending)
        .map(|k| Uint64::from(u64::from_be_bytes(k.unwrap())))
        .collect())
}

pub fn authenticator_by_id(store: &dyn Storage, id: Uint64) -> StdResult<String> {
    let auth = AUTHENTICATORS.load(store, id.u64().to_be_bytes())?;

    match serde_json::to_string(&auth) {
        Ok(auth_str) => Ok(auth_str),
        Err(error) => Err(StdError::GenericErr {msg: error.to_string()}),
    }

}

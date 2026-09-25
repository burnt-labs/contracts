use cosmwasm_std::{Addr, Empty, Order, StdError, StdResult, Storage};
use cw_storage_plus::Map;

/// Proxies allowed to send `ProxyExecute`. Kept deliberately small and bounded.
pub const TRUSTED_PROXIES: Map<&Addr, Empty> = Map::new("trusted_proxies");

/// Hard cap on registered proxies. Keeps every iteration over the map trivially bounded.
pub const MAX_TRUSTED_PROXIES: usize = 4;

pub fn list_trusted_proxies(storage: &dyn Storage) -> StdResult<Vec<Addr>> {
    TRUSTED_PROXIES
        .keys(storage, None, None, Order::Ascending)
        .take(MAX_TRUSTED_PROXIES)
        .collect()
}

pub fn count_trusted_proxies(storage: &dyn Storage) -> usize {
    TRUSTED_PROXIES
        .keys_raw(storage, None, None, Order::Ascending)
        .take(MAX_TRUSTED_PROXIES + 1)
        .count()
}

pub fn has_trusted_proxies(storage: &dyn Storage) -> bool {
    TRUSTED_PROXIES
        .keys_raw(storage, None, None, Order::Ascending)
        .next()
        .is_some()
}

/// Hard bound on how many entries a clear will ever touch. Far above anything the
/// registration API can produce; exists only so imported storage can never make this loop
/// unbounded. Exceeding it fails the migration as a whole (migrations are atomic, so
/// nothing is partially cleared); such a collection would have to be repaired by other
/// means before it can move onto this code. Unreachable through the normal API.
const MAX_CLEAR_ITERATIONS: usize = 256;

/// Remove every entry and verify the map is empty afterwards. Used when migrating from a
/// base `asset` contract so that dormant entries from an earlier life as `asset-proxyable`
/// cannot silently wake up. Does not rely on the cap: imported storage could hold more.
pub fn clear_trusted_proxies(storage: &mut dyn Storage) -> StdResult<usize> {
    let mut cleared = 0;
    loop {
        let keys: Vec<Addr> = TRUSTED_PROXIES
            .keys(storage, None, None, Order::Ascending)
            .take(MAX_TRUSTED_PROXIES)
            .collect::<StdResult<_>>()?;
        if keys.is_empty() {
            return Ok(cleared);
        }
        for key in &keys {
            TRUSTED_PROXIES.remove(storage, key);
        }
        cleared += keys.len();
        if cleared > MAX_CLEAR_ITERATIONS {
            return Err(StdError::generic_err(
                "trusted proxy map too large to clear during migration",
            ));
        }
    }
}

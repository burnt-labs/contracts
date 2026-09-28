use cosmwasm_std::{Addr, Empty, Order, StdError, StdResult, Storage};
use cw_storage_plus::Map;

/// Proxies allowed to send `ProxyExecute`. Kept deliberately small and bounded.
pub const TRUSTED_PROXIES: Map<&Addr, Empty> = Map::new("trusted_proxies");

/// Hard cap on registered proxies. Keeps every iteration over the map trivially bounded.
pub const MAX_TRUSTED_PROXIES: usize = 4;

/// Operators a trusted proxy may name in a relayed `ApproveAll`.
///
/// Trusting a proxy means trusting it to say who the sender is. Without this list that
/// also means trusting it to say *what* is approved, so a compromised proxy could make an
/// attacker operator over every holder in the collection. This bounds that to addresses
/// the creator chose, and it lives here rather than in the proxy so swapping the proxy
/// cannot sidestep it.
///
/// Empty means deny: a collection that has not listed an operator relays no approvals.
/// The base asset's `AllowedMarketplaces` plugin has the opposite convention, where empty
/// allows everything, which reads the wrong way round for a security control.
pub const APPROVAL_OPERATORS: Map<&Addr, Empty> = Map::new("approval_operators");

/// Hard cap on approvable operators, for the same reason as the proxy cap.
pub const MAX_APPROVAL_OPERATORS: usize = 8;

pub fn list_approval_operators(storage: &dyn Storage) -> StdResult<Vec<Addr>> {
    APPROVAL_OPERATORS
        .keys(storage, None, None, Order::Ascending)
        .take(MAX_APPROVAL_OPERATORS)
        .collect()
}

pub fn count_approval_operators(storage: &dyn Storage) -> usize {
    APPROVAL_OPERATORS
        .keys_raw(storage, None, None, Order::Ascending)
        .take(MAX_APPROVAL_OPERATORS + 1)
        .count()
}

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
/// unbounded. Exceeding it fails the whole calling transaction, migration or handover
/// alike, so nothing is ever partially cleared; such a collection would have to be
/// repaired by other means. Unreachable through the normal API.
const MAX_CLEAR_ITERATIONS: usize = 256;

/// Remove every entry and verify the map is empty afterwards. Called on both paths that
/// reset proxy trust: an accepted creator handover, and a migration whose source is the
/// base `asset` contract or which rotated the creator. The base-contract case is why this
/// does not rely on the cap -- imported storage could hold more entries than the
/// registration API allows, and dormant entries from an earlier life as `asset-proxyable`
/// must not silently wake up.
pub fn clear_trusted_proxies(storage: &mut dyn Storage) -> StdResult<usize> {
    clear_map(
        storage,
        &TRUSTED_PROXIES,
        MAX_TRUSTED_PROXIES,
        "trusted proxy",
    )
}

/// Same, for the approvable-operator set. A creator change clears both: the incoming
/// creator inherits no proxy-related trust of any kind.
pub fn clear_approval_operators(storage: &mut dyn Storage) -> StdResult<usize> {
    clear_map(
        storage,
        &APPROVAL_OPERATORS,
        MAX_APPROVAL_OPERATORS,
        "approval operator",
    )
}

fn clear_map(
    storage: &mut dyn Storage,
    map: &Map<&Addr, Empty>,
    batch: usize,
    what: &str,
) -> StdResult<usize> {
    let mut cleared = 0;
    loop {
        let keys: Vec<Addr> = map
            .keys(storage, None, None, Order::Ascending)
            .take(batch)
            .collect::<StdResult<_>>()?;
        if keys.is_empty() {
            return Ok(cleared);
        }
        for key in &keys {
            map.remove(storage, key);
        }
        cleared += keys.len();
        if cleared > MAX_CLEAR_ITERATIONS {
            return Err(StdError::generic_err(format!(
                "{what} map too large to clear"
            )));
        }
    }
}

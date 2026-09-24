use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Empty, Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Map};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub max_approval_seconds: Option<u64>,
}

/// A collection's standing on the allowlist.
#[cw_serde]
pub enum CollectionStatus {
    /// All sponsored actions.
    Active,
    /// Quarantined: only `RevokeAll` may be relayed, so users can still withdraw authority
    /// from an operator after an incident without an unsponsored transaction.
    RevocationOnly,
}

/// A collection entry as returned by the `collections` query.
#[cw_serde]
pub struct CollectionEntry {
    pub address: Addr,
    pub status: CollectionStatus,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ALLOWED_OPERATORS: Map<&Addr, Empty> = Map::new("allowed_operators");
/// Operators that were allowed once and then removed. `RevokeAll` stays available for them.
/// Bounded by the instantiate-time set, since operators can never be added.
pub const FORMER_OPERATORS: Map<&Addr, Empty> = Map::new("former_operators");
pub const COLLECTIONS: Map<&Addr, CollectionStatus> = Map::new("collections");

pub const MAX_ALLOWED_OPERATORS: usize = 4;
pub const MAX_COLLECTIONS: usize = 256;
/// Upper bound for `max_approval_seconds` (ten years). Keeps expiry arithmetic far from
/// any overflow and rules out a cap that is effectively "never".
pub const MAX_APPROVAL_CAP_SECONDS: u64 = 10 * 365 * 24 * 3600;

pub const DEFAULT_PAGE: u32 = 30;
pub const MAX_PAGE: u32 = 100;

pub fn list_operators(storage: &dyn Storage) -> StdResult<Vec<Addr>> {
    ALLOWED_OPERATORS
        .keys(storage, None, None, Order::Ascending)
        .take(MAX_ALLOWED_OPERATORS)
        .collect()
}

pub fn list_former_operators(storage: &dyn Storage) -> StdResult<Vec<Addr>> {
    FORMER_OPERATORS
        .keys(storage, None, None, Order::Ascending)
        .take(MAX_ALLOWED_OPERATORS)
        .collect()
}

pub fn count_collections(storage: &dyn Storage) -> usize {
    COLLECTIONS
        .keys_raw(storage, None, None, Order::Ascending)
        .take(MAX_COLLECTIONS + 1)
        .count()
}

pub fn list_collections(
    storage: &dyn Storage,
    start_after: Option<&Addr>,
    limit: Option<u32>,
) -> StdResult<Vec<CollectionEntry>> {
    let limit = limit.unwrap_or(DEFAULT_PAGE).min(MAX_PAGE) as usize;
    COLLECTIONS
        .range(
            storage,
            start_after.map(Bound::exclusive),
            None,
            Order::Ascending,
        )
        .take(limit)
        .map(|item| item.map(|(address, status)| CollectionEntry { address, status }))
        .collect()
}

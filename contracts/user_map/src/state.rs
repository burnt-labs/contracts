use cosmwasm_std::Addr;
use cw_storage_plus::Map;

/// Maximum number of bytes allowed in a stored `Update` value. Prevents
/// state bloat and the amplification of unbounded query responses.
pub const MAX_VALUE_LEN: usize = 8192;

/// Default page size for list queries when the caller omits `limit`.
pub const DEFAULT_QUERY_LIMIT: u32 = 50;

/// Hard cap on list-query page size, regardless of the requested `limit`.
pub const MAX_QUERY_LIMIT: u32 = 100;

#[allow(dead_code)]
pub const USER_MAP: Map<Addr, String> = Map::new("user_map");

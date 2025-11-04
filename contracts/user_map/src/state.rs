use cosmwasm_std::Addr;
use cw_storage_plus::Map;

#[allow(dead_code)]
pub const USER_MAP: Map<Addr, String> = Map::new("user_map");

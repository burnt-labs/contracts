use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use serde_json::Value;

pub const USER_MAP: Map<Addr, Value> = Map::new("user_map");

pub const OPACITY_VERIFIER: Item<Addr> = Item::new("opacity_verifier");
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const USER_MAP: Map<Addr, String> = Map::new("user_map");

pub const VERIFICATION_ADDR: Item<Addr> = Item::new("verification_addr");
pub const CLAIM_VALUE_KEY: Item<String> = Item::new("claim_value_key");
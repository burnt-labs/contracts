use cosmwasm_std::{Addr, Empty};
use cw_storage_plus::{Item, Map};

pub const VERIFICATION_KEY_ALLOW_LIST: Map<String, Empty> = Map::new("verification_key_allow_list");

pub const ADMIN: Item<Addr> = Item::new("admin");
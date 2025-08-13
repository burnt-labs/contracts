use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const ADMIN: Item<Option<Addr>> = Item::new("admin");
pub const CODE_IDS: Map<u64, bool> = Map::new("code_ids");

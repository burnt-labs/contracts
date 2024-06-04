use crate::grant::GrantConfig;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// msg_type_url to grant config
pub const GRANT_CONFIGS: Map<String, GrantConfig> = Map::new("grant_configs");

pub const ADMIN: Item<Addr> = Item::new("admin");

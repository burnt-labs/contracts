use crate::grant::{FeeConfig, GrantConfigStorage};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

// msg_type_url to grant config
pub const GRANT_CONFIGS: Map<String, GrantConfigStorage> = Map::new("grant_configs");

pub const FEE_CONFIG: Item<FeeConfig> = Item::new("fee_config");

pub const ADMIN: Item<Addr> = Item::new("admin");

pub const PENDING_ADMIN: Item<Addr> = Item::new("pending_admin");

#[cw_serde]
pub struct Params {
    pub display_url: String,
    pub redirect_url: String,
    pub icon_url: String,
}

pub const PARAMS: Item<Params> = Item::new("params");

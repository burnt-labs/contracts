use crate::grant::GrantConfig;
use cw_storage_plus::Map;

// msg_type_url to grant config
pub const GRANT_CONFIGS: Map<String, GrantConfig> = Map::new("grant_configs");

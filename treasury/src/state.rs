use cosmos_sdk_proto::Any;
use cw_storage_plus::{Map};
use crate::grant::{Authorization, GrantConfig};

pub const GRANTS: Map<Authorization, GrantConfig> = Map::new("grants");
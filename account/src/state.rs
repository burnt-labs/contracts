use crate::auth::Authenticator;
use cw_storage_plus::Map;

pub const AUTHENTICATORS: Map<u8, Authenticator> = Map::new("authenticators");

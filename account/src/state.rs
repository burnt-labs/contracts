use crate::auth::Authenticator;
use cosmwasm_std::{Uint64};
use cw_storage_plus::{Map};

pub const AUTHENTICATORS: Map<[u8; 8], Authenticator> = Map::new("authenticators");

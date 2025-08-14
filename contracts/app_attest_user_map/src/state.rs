use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub enum UserColor {
    Red,
    Green,
    Black,
    White,
}

#[cw_serde]
pub struct UserStatus {
    pub number: u8,
    pub color: UserColor,
}

pub const USER_MAP: Map<Addr, UserStatus> = Map::new("user_map");


pub const APP_ATTEST_VERIFICATION_ADDR: Item<Addr> = Item::new("app_attest_verification_addr");
pub const APP_ID: Item<String> = Item::new("app_id");
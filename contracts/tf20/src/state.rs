use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw20_base::state::MinterData;
use cw_storage_plus::Item;

#[cw_serde]
pub struct TokenInfo {
    pub creator: String,
    pub subdenom: String,
    pub admin: Option<Addr>,
}

pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");

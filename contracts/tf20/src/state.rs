use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[cw_serde]
pub struct TokenInfo {
    pub denom: String,
    pub admin: Option<Addr>,
}

pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");

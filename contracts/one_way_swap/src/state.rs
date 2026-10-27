use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    /// Set by ProposeAdmin; becomes admin when this address calls AcceptAdmin.
    pub pending_admin: Option<Addr>,
    pub in_denom: String,
    pub out_denom: String,
    /// out_denom base units paid per 1 in_denom base unit
    pub rate: Decimal,
}

pub const CONFIG: Item<Config> = Item::new("config");

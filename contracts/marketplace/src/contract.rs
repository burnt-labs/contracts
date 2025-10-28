use std::env;

use crate::error::ContractError;
use crate::events::{
    cancel_listing_event, create_listing_event, item_sold_event, pending_sale_created_event,
    sale_approved_event, sale_rejected_event, update_config_event,
};
use crate::helpers::{
    asset_buy_msg, asset_delist_msg, asset_list_msg, asset_reserve_msg, generate_id, not_listed,
    only_manager, only_owner, query_listing, valid_payment,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::offers::{
    execute_accept_collection_offer, execute_accept_offer, execute_cancel_collection_offer,
    execute_cancel_offer, execute_create_collection_offer, execute_create_offer,
};
use crate::state::init_auto_increment;
use crate::state::{listings, pending_sales, Listing, ListingStatus, PendingSale, SaleType};
use crate::state::{Config, CONFIG};
use cosmwasm_std::{
    ensure_eq, to_json_binary, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config::from_str(msg.config, deps.api)?;
    config.save(deps.storage)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    init_auto_increment(deps.storage)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

use crate::grant::GrantConfig;
use crate::state::GRANT_CONFIGS;
use cosmwasm_std::{Order, StdResult, Storage};

pub fn grant_config_type_urls(store: &dyn Storage) -> StdResult<Vec<String>> {
    Ok(GRANT_CONFIGS
        .keys(store, None, None, Order::Ascending)
        .map(|k| k.unwrap())
        .collect())
}

pub fn grant_config_by_type_url(
    store: &dyn Storage,
    msg_type_url: String,
) -> StdResult<GrantConfig> {
    GRANT_CONFIGS.load(store, msg_type_url)
}

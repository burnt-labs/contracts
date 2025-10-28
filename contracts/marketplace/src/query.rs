use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, Order, StdResult};
use cw_storage_plus::Bound;

use crate::msg::QueryMsg;
use crate::state::{
    collection_offers, listings, offers, pending_sales, CollectionOffer, Config, Listing, Offer,
    PendingSale, CONFIG,
};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(_deps)?),
        QueryMsg::Listing { listing_id } => to_json_binary(&query_listing(_deps, listing_id)?),
        QueryMsg::Offer { offer_id } => to_json_binary(&query_offer(_deps, offer_id)?),
        QueryMsg::CollectionOffer {
            collection_offer_id,
        } => to_json_binary(&query_collection_offer(_deps, collection_offer_id)?),
        QueryMsg::PendingSale { id } => to_json_binary(&query_pending_sale(_deps, id)?),
        QueryMsg::PendingSales { start_after, limit } => {
            to_json_binary(&query_pending_sales(_deps, start_after, limit)?)
        }
        QueryMsg::PendingSalesByExpiry { start_after, limit } => {
            to_json_binary(&query_pending_sales_by_expiry(_deps, start_after, limit)?)
        }
    }
}

pub fn query_pending_sale(deps: Deps, id: String) -> StdResult<PendingSale> {
    pending_sales().load(deps.storage, id)
}

pub fn query_pending_sales(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<PendingSale>> {
    let limit = limit.unwrap_or(30).min(30) as usize;
    let start = start_after.map(|v| Bound::exclusive(v.to_string()));

    pending_sales()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, sale)| sale))
        .collect::<StdResult<Vec<_>>>()
}

pub fn query_pending_sales_by_expiry(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<PendingSale>> {
    let limit = limit.unwrap_or(30).min(30) as usize;
    let start = start_after.map(|v| Bound::exclusive((v, "".to_string())));

    pending_sales()
        .idx
        .by_expiration
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, sale)| sale))
        .collect::<StdResult<Vec<_>>>()
}

pub fn query_config(deps: Deps) -> StdResult<Config<Addr>> {
    CONFIG.load(deps.storage)
}

pub fn query_listing(deps: Deps, listing_id: String) -> StdResult<Listing> {
    listings().load(deps.storage, listing_id)
}

pub fn query_offer(deps: Deps, offer_id: String) -> StdResult<Offer> {
    offers().load(deps.storage, offer_id)
}

pub fn query_collection_offer(
    deps: Deps,
    collection_offer_id: String,
) -> StdResult<CollectionOffer> {
    collection_offers().load(deps.storage, collection_offer_id)
}

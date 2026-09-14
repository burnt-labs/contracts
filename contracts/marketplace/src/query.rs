use cosmwasm_std::{to_json_binary, Addr, Binary, Deps, Env, Order, StdResult};
use cw_storage_plus::Bound;

use crate::msg::QueryMsg;
use crate::state::{
    collection_offers, listings, offers, pending_sales, CollectionOffer, Config, Listing, Offer,
    PendingSale, CONFIG,
};

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 100;

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(_deps)?),
        QueryMsg::Listing { listing_id } => to_json_binary(&query_listing(_deps, listing_id)?),
        QueryMsg::Listings { start_after, limit } => {
            to_json_binary(&query_listings(_deps, start_after, limit)?)
        }
        QueryMsg::ListingsBySeller {
            seller,
            start_after,
            limit,
        } => to_json_binary(&query_listings_by_seller(
            _deps,
            seller,
            start_after,
            limit,
        )?),
        QueryMsg::ListingsByCollection {
            collection,
            start_after,
            limit,
        } => to_json_binary(&query_listings_by_collection(
            _deps,
            collection,
            start_after,
            limit,
        )?),
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
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
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
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
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

pub fn query_listings(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Listing>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    listings()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, listing)| listing))
        .collect()
}

pub fn query_listings_by_seller(
    deps: Deps,
    seller: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Listing>> {
    let seller = deps.api.addr_validate(&seller)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    listings()
        .idx
        .by_seller
        .prefix(seller)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, listing)| listing))
        .collect()
}

pub fn query_listings_by_collection(
    deps: Deps,
    collection: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Listing>> {
    let collection = deps.api.addr_validate(&collection)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    listings()
        .idx
        .by_collection
        .prefix(collection)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(_, listing)| listing))
        .collect()
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

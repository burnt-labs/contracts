use cosmwasm_std::{entry_point, to_json_binary, Addr, Binary, Deps, Env, StdResult};

use crate::msg::QueryMsg;
use crate::state::{
    collection_offers, listings, offers, CollectionOffer, Config, Listing, Offer, CONFIG,
};

#[entry_point]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(_deps)?),
        QueryMsg::Listing { listing_id } => to_json_binary(&query_listing(_deps, listing_id)?),
        QueryMsg::Offer { offer_id } => to_json_binary(&query_offer(_deps, offer_id)?),
        QueryMsg::CollectionOffer {
            collection_offer_id,
        } => to_json_binary(&query_collection_offer(_deps, collection_offer_id)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config<Addr>> {
    Ok(CONFIG.load(deps.storage)?)
}

pub fn query_listing(deps: Deps, listing_id: String) -> StdResult<Listing> {
    Ok(listings().load(deps.storage, listing_id)?)
}

pub fn query_offer(deps: Deps, offer_id: String) -> StdResult<Offer> {
    Ok(offers().load(deps.storage, offer_id)?)
}

pub fn query_collection_offer(
    deps: Deps,
    collection_offer_id: String,
) -> StdResult<CollectionOffer> {
    Ok(collection_offers().load(deps.storage, collection_offer_id)?)
}

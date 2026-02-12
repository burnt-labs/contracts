use cosmwasm_std::{Addr, Coin, Event};

use crate::state::Config;

pub fn create_listing_event(
    id: String,
    owner: Addr,
    collection: Addr,
    token_id: String,
    price: Coin,
    reserved_for: Option<Addr>,
) -> Event {
    let reserved_for = reserved_for
        .map(|addr| addr.to_string())
        .unwrap_or("".to_string());
    Event::new(format!("{}/list-item", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("owner", owner.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("reserved_for", reserved_for.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("price", price.to_string())
}

pub fn update_config_event(config: Config<String>) -> Event {
    Event::new(format!("{}/update-config", env!("CARGO_PKG_NAME")))
        .add_attribute("manager", config.manager.to_string())
        .add_attribute("fee_recipient", config.fee_recipient.to_string())
        .add_attribute("fee_bps", config.fee_bps.to_string())
        .add_attribute("listing_denom", config.listing_denom.to_string())
        .add_attribute("sale_approvals", config.sale_approvals.to_string())
}
pub fn cancel_listing_event(id: String, collection: Addr, owner: Addr, token_id: String) -> Event {
    Event::new(format!("{}/cancel-listing", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("owner", owner.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
}

#[allow(clippy::too_many_arguments)]
pub fn item_sold_event(
    id: String,
    collection: Addr,
    seller: Addr,
    buyer: Addr,
    token_id: String,
    price: Coin,
    offer_id: Option<String>,
    collection_offer_id: Option<String>,
) -> Event {
    let mut sold_event = Event::new(format!("{}/item-sold", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("seller", seller.to_string())
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("price", price.to_string());
    if let Some(id) = offer_id {
        sold_event = sold_event.add_attribute("offer_id", id);
    }
    if let Some(id) = collection_offer_id {
        sold_event = sold_event.add_attribute("collection_offer_id", id);
    }
    sold_event
}

pub fn cancel_offer_event(id: String, collection: Addr, owner: Addr, token_id: String) -> Event {
    Event::new(format!("{}/cancel-offer", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("buyer", owner.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
}

pub fn cancel_collection_offer_event(id: String, collection: Addr, owner: Addr) -> Event {
    Event::new(format!(
        "{}/cancel-collection-offer",
        env!("CARGO_PKG_NAME")
    ))
    .add_attribute("id", id)
    .add_attribute("buyer", owner.to_string())
    .add_attribute("collection", collection.to_string())
}

pub fn create_offer_event(
    id: String,
    collection: Addr,
    buyer: Addr,
    token_id: String,
    price: Coin,
) -> Event {
    Event::new(format!("{}/create-offer", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("price", price.to_string())
}

pub fn create_collection_offer_event(
    id: String,
    collection: Addr,
    owner: Addr,
    price: Coin,
) -> Event {
    Event::new(format!(
        "{}/create-collection-offer",
        env!("CARGO_PKG_NAME")
    ))
    .add_attribute("id", id)
    .add_attribute("owner", owner.to_string())
    .add_attribute("collection", collection.to_string())
    .add_attribute("price", price.to_string())
}

pub fn pending_sale_created_event(
    id: String,
    collection: Addr,
    token_id: String,
    buyer: Addr,
    seller: Addr,
    price: Coin,
) -> Event {
    Event::new(format!("{}/pending-sale-created", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("seller", seller.to_string())
        .add_attribute("price", price.to_string())
}

pub fn sale_approved_event(
    pending_sale_id: String,
    collection: Addr,
    token_id: String,
    buyer: Addr,
    seller: Addr,
    price: Coin,
) -> Event {
    Event::new(format!("{}/sale-approved", env!("CARGO_PKG_NAME")))
        .add_attribute("pending_sale_id", pending_sale_id)
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("seller", seller.to_string())
        .add_attribute("price", price.to_string())
}

pub fn sale_rejected_event(
    pending_sale_id: String,
    collection: Addr,
    token_id: String,
    buyer: Addr,
    seller: Addr,
    price: Coin,
    reason: &str,
) -> Event {
    Event::new(format!("{}/sale-rejected", env!("CARGO_PKG_NAME")))
        .add_attribute("pending_sale_id", pending_sale_id)
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("seller", seller.to_string())
        .add_attribute("price", price.to_string())
        .add_attribute("reason", reason)
}

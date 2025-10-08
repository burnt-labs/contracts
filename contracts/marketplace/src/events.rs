use cosmwasm_std::{Addr, Coin, Event};

pub fn create_listing_event(
    id: String,
    owner: Addr,
    collection: Addr,
    token_id: String,
    price: Coin,
) -> Event {
    Event::new(format!("{}/list-item", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("owner", owner.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("price", price.to_string())
}

pub fn cancel_listing_event(id: String, collection: Addr, owner: Addr, token_id: String) -> Event {
    Event::new(format!("{}/cancel-listing", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("owner", owner.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
}

pub fn item_sold_event(
    id: String,
    collection: Addr,
    seller: Addr,
    buyer: Addr,
    token_id: String,
    price: Coin,
) -> Event {
    Event::new(format!("{}/item-sold", env!("CARGO_PKG_NAME")))
        .add_attribute("id", id)
        .add_attribute("seller", seller.to_string())
        .add_attribute("buyer", buyer.to_string())
        .add_attribute("collection", collection.to_string())
        .add_attribute("token_id", token_id)
        .add_attribute("price", price.to_string())
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

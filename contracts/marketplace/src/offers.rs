use crate::error::ContractError;
use crate::helpers::{generate_id, valid_payment};
use crate::state::{Offer, CONFIG};
use cosmwasm_std::{Addr, Coin, DepsMut, MessageInfo, Response};

use crate::state::{next_auto_increment, offers};

pub fn execute_create_offer(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    valid_payment(&info, price.clone(), config.listing_denom)?;
    let auto_increment = next_auto_increment(deps.storage)?;
    let id = generate_id(vec![
        &collection.as_bytes(),
        &token_id.as_bytes(),
        &info.sender.as_bytes(),
        &auto_increment.to_string().as_bytes(),
    ]);
    let offer = Offer {
        id: id.to_string(),
        buyer: info.sender,
        collection,
        token_id,
        price,
    };
    // reject offer for potential collision
    offers().update(deps.storage, id.to_string(), |prev| match prev {
        Some(_) => Err(ContractError::OfferAlreadyExists { id: id.to_string() }),
        None => Ok(offer),
    })?;
    Ok(Response::new().add_attribute("method", "create_offer"))
}
pub fn execute_accept_offer(
    deps: DepsMut,
    info: MessageInfo,
    offer_id: String,
    collection: Addr,
    token_id: String,
    price: Coin,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "accept_offer"))
}

pub fn execute_cancel_offer(
    deps: DepsMut,
    info: MessageInfo,
    offer_id: String,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "cancel_offer"))
}

pub fn execute_create_collection_offer(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "create_collection_offer"))
}

pub fn execute_accept_collection_offer(
    deps: DepsMut,
    info: MessageInfo,
    offer_id: String,
    collection: Addr,
    token_id: String,
    price: Coin,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "accept_collection_offer"))
}

pub fn execute_cancel_collection_offer(
    deps: DepsMut,
    info: MessageInfo,
    offer_id: String,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "cancel_collection_offer"))
}

use crate::error::ContractError;
use crate::helpers::{generate_id, only_owner, valid_payment};
use crate::state::{collection_offers, CollectionOffer, Offer, CONFIG};

use cosmwasm_std::{ensure_eq, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response};

use crate::events::{
    cancel_collection_offer_event, cancel_offer_event, create_collection_offer_event,
    create_offer_event,
};
use crate::state::{next_auto_increment, offers};

pub fn execute_create_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // ensure valid payment is sent for escrow
    valid_payment(&info, price.clone(), config.listing_denom)?;
    let auto_increment = next_auto_increment(deps.storage)?;
    let id = generate_id(vec![
        env.block.height.to_string().as_bytes(),
        &auto_increment.to_string().as_bytes(),
        &collection.as_bytes(),
        &token_id.as_bytes(),
        &info.sender.as_bytes(),
    ]);
    let offer = Offer {
        id: id.clone().to_string(),
        buyer: info.sender.clone(),
        collection: collection.clone(),
        token_id: token_id.clone(),
        price: price.clone(),
    };
    // reject offer for potential collision
    offers().update(deps.storage, id.clone().to_string(), |prev| match prev {
        Some(_) => Err(ContractError::OfferAlreadyExists { id: id.to_string() }),
        None => Ok(offer),
    })?;

    Ok(Response::new().add_event(create_offer_event(
        id,
        collection,
        info.sender,
        token_id,
        price,
    )))
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
    let offer = offers().load(deps.storage, offer_id.clone())?;
    ensure_eq!(
        offer.buyer,
        info.sender,
        ContractError::Unauthorized {
            message: "sender is not the buyer".to_string()
        }
    );
    offers().remove(deps.storage, offer_id)?;
    let refund_msg = BankMsg::Send {
        to_address: offer.buyer.to_string(),
        amount: vec![offer.price],
    };
    Ok(Response::new()
        .add_event(cancel_offer_event(
            offer.id,
            offer.collection,
            offer.buyer,
            offer.token_id,
        ))
        .add_message(refund_msg))
}

pub fn execute_create_collection_offer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // ensure valid payment is sent for escrow
    valid_payment(&info, price.clone(), config.listing_denom)?;
    let auto_increment = next_auto_increment(deps.storage)?;
    let id = generate_id(vec![
        env.block.height.to_string().as_bytes(),
        &auto_increment.to_string().as_bytes(),
        &collection.as_bytes(),
        &info.sender.as_bytes(),
    ]);
    let collection_offer = CollectionOffer {
        id: id.to_string(),
        buyer: info.sender.clone(),
        collection: collection.clone(),
        price: price.clone(),
    };
    // reject offer for potential collision
    collection_offers().update(deps.storage, id.to_string(), |prev| match prev {
        Some(_) => Err(ContractError::OfferAlreadyExists { id: id.to_string() }),
        None => Ok(collection_offer),
    })?;
    Ok(Response::new().add_event(create_collection_offer_event(
        id,
        collection,
        info.sender,
        price,
    )))
}

pub fn execute_accept_collection_offer(
    deps: DepsMut,
    info: MessageInfo,
    offer_id: String,
    collection: Addr,
    token_id: String,
    price: Coin,
) -> Result<Response, ContractError> {
    // only owner of the nft can accept the offer
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    let offer = offers().load(deps.storage, offer_id)?;
    ensure_eq!(offer.buyer, info.sender, ContractError::InvalidSeller {});
    ensure_eq!(
        offer.price,
        price,
        ContractError::InvalidPrice {
            expected: offer.price,
            actual: price
        }
    );

    Ok(Response::new().add_attribute("method", "accept_collection_offer"))
}

pub fn execute_cancel_collection_offer(
    deps: DepsMut,
    info: MessageInfo,
    offer_id: String,
) -> Result<Response, ContractError> {
    let collection_offer = collection_offers().load(deps.storage, offer_id.clone())?;
    ensure_eq!(
        collection_offer.buyer,
        info.sender,
        ContractError::Unauthorized {
            message: "sender is not the buyer".to_string()
        }
    );
    collection_offers().remove(deps.storage, offer_id)?;

    let refund_msg = BankMsg::Send {
        to_address: collection_offer.buyer.to_string(),
        amount: vec![collection_offer.price],
    };
    Ok(Response::new()
        .add_event(cancel_collection_offer_event(
            collection_offer.id,
            collection_offer.collection,
            collection_offer.buyer,
        ))
        .add_message(refund_msg))
}

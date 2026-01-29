use crate::error::ContractError;
use crate::events::item_sold_event;
use crate::helpers::{
    asset_buy_msg, asset_list_msg, calculate_asset_price, generate_id, only_owner, valid_payment,
};
use crate::state::{collection_offers, CollectionOffer, Offer, CONFIG};
use cosmwasm_std::{
    ensure_eq, to_json_binary, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, WasmMsg,
};

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

    // Disable offers when sale approvals are enabled
    if config.sale_approvals {
        return Err(ContractError::OfferesDisabled {});
    }

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
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    let config = CONFIG.load(deps.storage)?;

    // Disable offer acceptance when sale approvals are enabled
    if config.sale_approvals {
        return Err(ContractError::OfferesDisabled {});
    }

    let offer = offers().load(deps.storage, offer_id.clone())?;
    ensure_eq!(
        offer.collection,
        collection,
        ContractError::InvalidCollection {
            expected: collection.clone().to_string(),
            actual: offer.collection.clone().to_string()
        }
    );
    ensure_eq!(
        token_id,
        offer.token_id,
        ContractError::InvalidTokenId {
            expected: offer.token_id.clone(),
            actual: token_id.clone()
        }
    );

    if offer.price != price {
        return Err(ContractError::InvalidPrice {
            expected: offer.price,
            actual: price,
        });
    }
    if offer.buyer == info.sender {
        return Err(ContractError::InvalidSeller {});
    }

    // Calculate asset price (seller proceeds) and marketplace fee
    // This ensures offer acceptance matches the immediate buy path
    let asset_price = calculate_asset_price(offer.price.clone(), config.fee_bps)?;
    let marketplace_fee_amount = offer
        .price
        .amount
        .checked_sub(asset_price.amount)
        .map_err(|_| ContractError::InsuficientFunds {})?;

    // list the item on the asset contract with asset_price (not full price)
    let list_msg = asset_list_msg(token_id.clone(), asset_price.clone(), None);
    // do a buy on the asset contract for the specific price and buyer
    let buy_msg = asset_buy_msg(offer.buyer.clone(), token_id.clone());

    offers().remove(deps.storage, offer_id.clone())?;

    let mut response = Response::new()
        .add_event(item_sold_event(
            "listing_id".to_string(),
            offer.collection.clone(),
            info.sender.clone(),
            offer.buyer.clone(),
            token_id.clone(),
            offer.price.clone(),
            Some(offer.id),
            None,
        ))
        .add_message(WasmMsg::Execute {
            contract_addr: offer.collection.clone().to_string(),
            msg: to_json_binary(&list_msg)?,
            funds: vec![],
        })
        // Send asset_price to asset contract (seller proceeds)
        .add_message(WasmMsg::Execute {
            contract_addr: offer.collection.clone().to_string(),
            msg: to_json_binary(&buy_msg)?,
            funds: vec![asset_price],
        });

    // Only send marketplace fee if it's greater than zero
    // CosmWasm doesn't allow sending empty coin amounts
    if !marketplace_fee_amount.is_zero() {
        response = response.add_message(BankMsg::Send {
            to_address: config.fee_recipient.to_string(),
            amount: vec![Coin {
                denom: offer.price.denom,
                amount: marketplace_fee_amount,
            }],
        });
    }

    Ok(response)
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

    // Disable collection offers when sale approvals are enabled
    if config.sale_approvals {
        return Err(ContractError::OfferesDisabled {});
    }

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
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    let config = CONFIG.load(deps.storage)?;

    // Disable collection offer acceptance when sale approvals are enabled
    if config.sale_approvals {
        return Err(ContractError::OfferesDisabled {});
    }

    let offer = collection_offers().load(deps.storage, offer_id.clone())?;
    ensure_eq!(
        offer.collection,
        collection,
        ContractError::InvalidCollection {
            expected: collection.clone().to_string(),
            actual: offer.collection.clone().to_string()
        }
    );

    if offer.price != price {
        return Err(ContractError::InvalidPrice {
            expected: offer.price,
            actual: price,
        });
    }
    if offer.buyer == info.sender {
        return Err(ContractError::InvalidSeller {});
    }

    // Calculate asset price (seller proceeds) and marketplace fee
    // This ensures collection offer acceptance matches the immediate buy path
    let asset_price = calculate_asset_price(offer.price.clone(), config.fee_bps)?;
    let marketplace_fee_amount = offer
        .price
        .amount
        .checked_sub(asset_price.amount)
        .map_err(|_| ContractError::InsuficientFunds {})?;

    // list the item on the asset contract with asset_price (not full price)
    let list_msg = asset_list_msg(token_id.clone(), asset_price.clone(), None);
    // do a buy on the asset contract for the specific price and buyer
    let buy_msg = asset_buy_msg(offer.buyer.clone(), token_id.clone());

    collection_offers().remove(deps.storage, offer_id.clone())?;

    let mut response = Response::new()
        .add_event(item_sold_event(
            "listing_id".to_string(),
            offer.collection.clone(),
            info.sender.clone(),
            offer.buyer.clone(),
            token_id.clone(),
            offer.price.clone(),
            None,
            Some(offer.id),
        ))
        .add_message(WasmMsg::Execute {
            contract_addr: offer.collection.clone().to_string(),
            msg: to_json_binary(&list_msg)?,
            funds: vec![],
        })
        // Send asset_price to asset contract (seller proceeds)
        .add_message(WasmMsg::Execute {
            contract_addr: offer.collection.clone().to_string(),
            msg: to_json_binary(&buy_msg)?,
            funds: vec![asset_price],
        });

    // Only send marketplace fee if it's greater than zero
    // CosmWasm doesn't allow sending empty coin amounts
    if !marketplace_fee_amount.is_zero() {
        response = response.add_message(BankMsg::Send {
            to_address: config.fee_recipient.to_string(),
            amount: vec![Coin {
                denom: offer.price.denom,
                amount: marketplace_fee_amount,
            }],
        });
    }

    Ok(response)
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

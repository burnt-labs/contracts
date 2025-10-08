use std::env;

use crate::error::ContractError;
use crate::helpers::{
    asset_buy_msg, asset_list_msg, generate_id, not_listed, only_owner, query_listing,
    valid_payment,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::offers::{
    execute_accept_collection_offer, execute_accept_offer, execute_cancel_collection_offer,
    execute_cancel_offer, execute_create_collection_offer, execute_create_offer,
};
use crate::state::init_auto_increment;
use crate::state::{listings, Listing, ListingStatus};
use crate::state::{Config, CONFIG};
use asset::msg::AssetExtensionExecuteMsg as AssetExecuteMsg;
use cosmwasm_std::{
    ensure_eq, entry_point, to_json_binary, Addr, Coin, DepsMut, Env, MessageInfo, Response,
    WasmMsg,
};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
#[entry_point]
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

use crate::events::{cancel_listing_event, create_listing_event, item_sold_event};
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::ListItem {
            collection,
            price,
            token_id,
        } => execute_create_listing(deps, info, api.addr_validate(&collection)?, price, token_id),
        ExecuteMsg::CancelListing { listing_id } => execute_cancel_listing(deps, info, listing_id),
        ExecuteMsg::BuyItem { listing_id, price } => {
            execute_buy_item(deps, info, listing_id, price)
        }
        ExecuteMsg::CreateOffer {
            collection,
            price,
            token_id,
        } => execute_create_offer(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            price,
            token_id,
        ),
        ExecuteMsg::AcceptOffer {
            id,
            collection,
            token_id,
            price,
        } => execute_accept_offer(
            deps,
            info,
            id,
            api.addr_validate(&collection)?,
            token_id,
            price,
        ),
        ExecuteMsg::CancelOffer { id } => execute_cancel_offer(deps, info, id),
        ExecuteMsg::CreateCollectionOffer { collection, price } => {
            execute_create_collection_offer(deps, env, info, api.addr_validate(&collection)?, price)
        }
        ExecuteMsg::AcceptCollectionOffer {
            id,
            collection,
            token_id,
            price,
        } => execute_accept_collection_offer(
            deps,
            info,
            id,
            api.addr_validate(&collection)?,
            token_id,
            price,
        ),

        ExecuteMsg::CancelCollectionOffer { id } => execute_cancel_collection_offer(deps, info, id),
    }
}

pub fn execute_create_listing(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
    token_id: String,
) -> Result<Response, ContractError> {
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    not_listed(&deps.querier, &collection, &token_id)?;
    let config = CONFIG.load(deps.storage)?;
    ensure_eq!(
        price.denom,
        CONFIG.load(deps.storage)?.listing_denom,
        ContractError::InvalidListingDenom {
            expected: config.listing_denom,
            actual: price.denom,
        }
    );

    // generate consistent id even across relisting helps single lookup
    let id = generate_id(vec![&collection.as_bytes(), &token_id.as_bytes()]);
    let listing = Listing {
        id: id.clone(),
        seller: info.sender.clone(),
        collection: collection.clone(),
        token_id: token_id.clone(),
        price: price.clone(),
        status: ListingStatus::Active,
    };
    // reject if listing already exists
    listings().update(deps.storage, id.clone(), |prev| match prev {
        Some(_) => Err(ContractError::AlreadyListed {}),
        None => Ok(listing),
    })?;
    let list_msg = asset_list_msg(token_id.clone(), price.clone());
    Ok(Response::new()
        .add_event(create_listing_event(
            id,
            info.sender,
            collection.clone(),
            token_id,
            price,
        ))
        .add_message(WasmMsg::Execute {
            contract_addr: collection.to_string(),
            msg: to_json_binary(&list_msg)?,
            funds: vec![],
        }))
}

pub fn execute_cancel_listing(
    deps: DepsMut,
    info: MessageInfo,
    listing_id: String,
) -> Result<Response, ContractError> {
    let listing = listings().load(deps.storage, listing_id.clone())?;
    ensure_eq!(
        listing.seller,
        info.sender,
        ContractError::Unauthorized {
            message: "sender is not the seller".to_string(),
        }
    );
    // can't cancel a list that is pending approval if sale approvals are enabled
    // listings that are in pending status have already been placed a matching buy order
    // but it's not yet been accepted by the manager
    if CONFIG.load(deps.storage)?.sale_approvals && listing.status != ListingStatus::Active {
        return Err(ContractError::InvalidListingStatus {
            expected: ListingStatus::Active.to_string(),
            actual: listing.status.to_string(),
        });
    }

    listings().remove(deps.storage, listing_id.clone())?;
    // query if there is a listing in the asset contract (in case is out of sync)
    let asset_listing = query_listing(&deps.querier, &listing.collection, &listing.token_id);

    let mut sub_msgs = vec![];

    if let Ok(_) = asset_listing {
        let cancel_listing = asset::msg::ExecuteMsg::<
            cw721::DefaultOptionalNftExtensionMsg,
            cw721::DefaultOptionalCollectionExtensionMsg,
            asset::msg::AssetExtensionExecuteMsg,
        >::UpdateExtension {
            msg: asset::msg::AssetExtensionExecuteMsg::Delist {
                token_id: listing.token_id.clone(),
            },
        };
        sub_msgs.push(WasmMsg::Execute {
            contract_addr: listing.collection.to_string(),
            msg: to_json_binary(&cancel_listing)?,
            funds: vec![],
        });
    }
    Ok(Response::new()
        .add_event(cancel_listing_event(
            listing_id,
            listing.collection.clone(),
            listing.seller,
            listing.token_id,
        ))
        .add_messages(sub_msgs))
}

pub fn execute_buy_item(
    deps: DepsMut,
    info: MessageInfo,
    listing_id: String,
    price: Coin,
) -> Result<Response, ContractError> {
    let listing = listings().load(deps.storage, listing_id.clone())?;
    // prevent price mismatch due to possible frontrunning
    if listing.price != price {
        return Err(ContractError::InvalidPrice {
            expected: listing.price,
            actual: price,
        });
    }

    // check payment and funds are valid
    valid_payment(&info, price.clone(), listing.price.denom)?;

    let buy_msg = asset_buy_msg(info.sender.clone(), listing.token_id.clone());

    Ok(Response::new()
        .add_event(item_sold_event(
            listing.id,
            listing.collection.clone(),
            listing.seller,
            info.sender,
            listing.token_id.clone(),
            price,
            None,
            None,
        ))
        .add_message(WasmMsg::Execute {
            contract_addr: listing.collection.clone().to_string(),
            msg: to_json_binary(&buy_msg)?,
            // send the payment to the asset contract
            funds: info.funds,
        }))
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

use std::env;

use crate::error::ContractError;
use crate::events::{
    cancel_listing_event, create_listing_event, item_sold_event, pending_sale_created_event,
    sale_approved_event, sale_rejected_event, update_config_event,
};
use crate::helpers::{
    asset_buy_msg, asset_delist_msg, asset_list_msg, asset_reserve_msg, generate_id, not_listed,
    only_manager, only_owner, query_listing, valid_payment,
};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::offers::{
    execute_accept_collection_offer, execute_accept_offer, execute_cancel_collection_offer,
    execute_cancel_offer, execute_create_collection_offer, execute_create_offer,
};
use crate::state::init_auto_increment;
use crate::state::{listings, pending_sales, Listing, ListingStatus, PendingSale, SaleType};
use crate::state::{Config, CONFIG};
use cosmwasm_std::{
    ensure_eq, to_json_binary, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, WasmMsg,
};
use cw2::set_contract_version;

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
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
            execute_buy_item(deps, env, info, listing_id, price, info.sender.clone())
        }
        ExecuteMsg::FinalizeFor {
            listing_id,
            price,
            recipient,
        } => execute_finalize_for(
            deps,
            info,
            listing_id,
            price,
            api.addr_validate(&recipient)?,
        ),
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
        ExecuteMsg::ApproveSale { id } => execute_approve_sale(deps, info, id),
        ExecuteMsg::RejectSale { id } => execute_reject_sale(deps, info, id),
        ExecuteMsg::UpdateConfig { config } => execute_update_config(deps, info, config),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    config: Config<String>,
) -> Result<Response, ContractError> {
    only_manager(&info, &deps)?;
    config.validate()?;
    let addr_config = config.to_addr(deps.api)?;
    CONFIG.save(deps.storage, &addr_config)?;
    Ok(Response::new().add_event(update_config_event(config)))
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
    let list_msg = asset_list_msg(
        token_id.clone(),
        price.clone(),
        Some(config.fee_bps as u16),
        Some(config.fee_recipient.to_string()),
    );
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

    if asset_listing.is_ok() {
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
    env: Env,
    info: MessageInfo,
    listing_id: String,
    price: Coin,
    recipient: Addr,
) -> Result<Response, ContractError> {
}
pub fn execute_buy_item(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    listing_id: String,
    price: Coin,
    recipient: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let listing = listings().load(deps.storage, listing_id.clone())?;

    // Prevent price mismatch due to possible frontrunning
    if listing.price != price {
        return Err(ContractError::InvalidPrice {
            expected: listing.price,
            actual: price,
        });
    }

    // Check payment and funds are valid
    valid_payment(&info, price.clone(), listing.price.denom.clone())?;

    // if approvals are enabled, create pending sale.
    if config.sale_approvals {
        return execute_create_pending_sale(deps, env, info, listing_id, listing, price);
    }

    // remove listing
    listings().remove(deps.storage, listing_id.clone())?;

    let buy_msg = asset_buy_msg(recipient, listing.token_id.clone());

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
            funds: info.funds,
        }))
}

fn execute_create_pending_sale(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    listing_id: String,
    listing: Listing,
    price: Coin,
) -> Result<Response, ContractError> {
    let pending_sale_id = generate_id(vec![
        listing_id.as_bytes(),
        info.sender.as_bytes(),
        &env.block.height.to_string().as_bytes(),
    ]);

    let pending_sale = PendingSale {
        id: pending_sale_id.clone(),
        collection: listing.collection.clone(),
        token_id: listing.token_id.clone(),
        price: price.clone(),
        seller: listing.seller.clone(),
        buyer: info.sender.clone(),
        sale_type: SaleType::BuyNow,
        time: env.block.time.seconds(),
        expiration: env.block.time.seconds() + 86400, // 24 hours
    };

    pending_sales().save(deps.storage, pending_sale_id.clone(), &pending_sale)?;

    // update listing status to reserved
    listings().update(deps.storage, listing_id.clone(), |l| match l {
        Some(mut listing) => {
            listing.status = ListingStatus::Reserved;
            Ok(listing)
        }
        None => Err(ContractError::ListingNotFound { id: listing_id }),
    })?;

    // Reserve the NFT in the asset contract
    let reserve_msg = asset_reserve_msg(
        listing.token_id.clone(),
        info.sender.clone(),
        cw721::Expiration::AtTime(env.block.time.plus_seconds(86400)),
    );

    // Funds are escrowed in contract (sent by buyer in info.funds)
    Ok(Response::new()
        .add_event(pending_sale_created_event(
            pending_sale_id,
            listing.collection.clone(),
            listing.token_id,
            info.sender,
            listing.seller,
            price,
        ))
        .add_message(WasmMsg::Execute {
            contract_addr: listing.collection.to_string(),
            msg: to_json_binary(&reserve_msg)?,
            funds: vec![],
        })
        .add_attribute("action", "pending_sale_created"))
}

pub fn execute_approve_sale(
    deps: DepsMut,
    info: MessageInfo,
    pending_sale_id: String,
) -> Result<Response, ContractError> {
    // Only manager can approve
    only_manager(&info, &deps)?;

    let pending_sale = pending_sales().load(deps.storage, pending_sale_id.clone())?;

    // Generate listing_id to find the listing
    let listing_id = generate_id(vec![
        pending_sale.collection.as_bytes(),
        pending_sale.token_id.as_bytes(),
    ]);

    // Execute the buy on asset contract
    let buy_msg = asset_buy_msg(pending_sale.buyer.clone(), pending_sale.token_id.clone());

    // delete original listing
    listings().remove(deps.storage, listing_id.clone())?;

    // remove from queue
    pending_sales().remove(deps.storage, pending_sale_id.clone())?;

    Ok(Response::new()
        .add_event(sale_approved_event(
            pending_sale_id,
            pending_sale.collection.clone(),
            pending_sale.token_id.clone(),
            pending_sale.buyer.clone(),
            pending_sale.seller.clone(),
            pending_sale.price.clone(),
        ))
        .add_event(item_sold_event(
            listing_id,
            pending_sale.collection.clone(),
            pending_sale.seller,
            pending_sale.buyer,
            pending_sale.token_id.clone(),
            pending_sale.price.clone(),
            None,
            None,
        ))
        .add_message(WasmMsg::Execute {
            contract_addr: pending_sale.collection.to_string(),
            msg: to_json_binary(&buy_msg)?,
            funds: vec![pending_sale.price],
        }))
}

pub fn execute_reject_sale(
    deps: DepsMut,
    info: MessageInfo,
    pending_sale_id: String,
) -> Result<Response, ContractError> {
    // Only manager can reject
    only_manager(&info, &deps)?;

    let pending_sale = pending_sales().load(deps.storage, pending_sale_id.clone())?;

    let listing_id = generate_id(vec![
        pending_sale.collection.as_bytes(),
        pending_sale.token_id.as_bytes(),
    ]);

    // delete the listing
    listings().remove(deps.storage, listing_id)?;

    // delist from asset contract
    let delist_msg = asset_delist_msg(pending_sale.token_id.clone());

    // refund buyer
    let refund_msg = BankMsg::Send {
        to_address: pending_sale.buyer.to_string(),
        amount: vec![pending_sale.price.clone()],
    };

    // remove pending sale from the queue
    pending_sales().remove(deps.storage, pending_sale_id.clone())?;

    Ok(Response::new()
        .add_event(sale_rejected_event(
            pending_sale_id,
            pending_sale.collection.clone(),
            pending_sale.token_id,
            pending_sale.buyer.clone(),
            pending_sale.seller,
            pending_sale.price,
        ))
        .add_message(WasmMsg::Execute {
            contract_addr: pending_sale.collection.to_string(),
            msg: to_json_binary(&delist_msg)?,
            funds: vec![],
        })
        .add_message(refund_msg))
}

use crate::error::ContractError;
use crate::events::{
    cancel_listing_event, create_listing_event, item_sold_event, pending_sale_created_event,
    sale_approved_event, sale_rejected_event, update_config_event,
};
use crate::helpers::{
    asset_buy_msg, asset_delist_msg, asset_list_msg, asset_reserve_msg, asset_unreserve_msg,
    generate_id, not_listed, only_manager, only_owner, query_listing, valid_payment,
};
use crate::msg::ExecuteMsg;
use crate::offers::{
    execute_accept_collection_offer, execute_accept_offer, execute_cancel_collection_offer,
    execute_cancel_offer, execute_create_collection_offer, execute_create_offer,
};

use crate::helpers::calculate_asset_price;
use crate::state::{listings, pending_sales, Listing, ListingStatus, PendingSale, SaleType};
use crate::state::{Config, CONFIG};
use asset::msg::ReserveMsg;
use cosmwasm_std::{
    ensure_eq, to_json_binary, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, Response, Timestamp,
    WasmMsg,
};
use cw_utils::maybe_addr;
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
            reserved_for,
        } => execute_create_listing(
            deps,
            env,
            info,
            api.addr_validate(&collection)?,
            price,
            token_id,
            maybe_addr(api, reserved_for)?,
        ),
        ExecuteMsg::CancelListing { listing_id } => execute_cancel_listing(deps, info, listing_id),
        ExecuteMsg::BuyItem { listing_id, price } => {
            execute_buy_item(deps, env, info.clone(), listing_id, price, info.sender)
        }
        ExecuteMsg::FinalizeFor {
            listing_id,
            price,
            recipient,
        } => execute_buy_item(
            deps,
            env,
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
        ExecuteMsg::ApproveSale { id } => execute_approve_sale(deps, env, info, id),
        ExecuteMsg::RejectSale { id } => execute_reject_sale(deps, info, id),
        ExecuteMsg::ReclaimExpiredSale { id } => execute_reclaim_expired_sale(deps, env, info, id),
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
    env: Env,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
    token_id: String,
    reserved_for: Option<Addr>,
) -> Result<Response, ContractError> {
    only_owner(&deps.querier, &info, &collection, &token_id)?;
    not_listed(&deps.querier, &collection, &token_id)?;
    let config = CONFIG.load(deps.storage)?;
    if price.amount.is_zero() {
        return Err(ContractError::InvalidPrice {
            expected: price.clone(),
            actual: price,
        });
    }

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
    let asset_price = calculate_asset_price(price.clone(), config.fee_bps)?;
    // if reserved is provided, use the contract address (the escrower) used to reserve in the asset contract

    let reservation = if reserved_for.clone().is_some() {
        Some(ReserveMsg {
            reserver: Some(env.contract.address.to_string()),
            reserved_until: Timestamp::from_seconds(env.block.time.seconds() + 365 * 24 * 60 * 60),
        })
    } else {
        None
    };
    let listing = Listing {
        id: id.clone(),
        seller: info.sender.clone(),
        collection: collection.clone(),
        token_id: token_id.clone(),
        price: price.clone(),
        asset_price: asset_price.clone(),
        reserved_for: reserved_for.clone(),
        status: ListingStatus::Active,
    };
    // reject if listing already exists
    listings().update(deps.storage, id.clone(), |prev| match prev {
        Some(_) => Err(ContractError::AlreadyListed {}),
        None => Ok(listing),
    })?;
    let list_msg = asset_list_msg(token_id.clone(), asset_price, reservation);
    Ok(Response::new()
        .add_event(create_listing_event(
            id,
            info.sender,
            collection.clone(),
            token_id,
            price,
            reserved_for.clone(),
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
        let cancel_listing = asset_delist_msg(listing.token_id.clone());
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
    let config = CONFIG.load(deps.storage)?;
    let listing = listings().load(deps.storage, listing_id.clone())?;

    if listing.status != ListingStatus::Active {
        return Err(ContractError::InvalidListingStatus {
            expected: ListingStatus::Active.to_string(),
            actual: listing.status.to_string(),
        });
    }

    if let Some(reserved_for) = listing.reserved_for.clone() {
        ensure_eq!(
            reserved_for,
            info.sender,
            ContractError::Unauthorized {
                message: "item is reserved for another address".to_string(),
            }
        );
    }
    // Prevent price mismatch due to possible frontrunning
    if listing.price != price {
        return Err(ContractError::InvalidPrice {
            expected: listing.price,
            actual: price,
        });
    }

    // Check payment and funds are valid
    let payment = valid_payment(&info, price.clone(), listing.price.denom.clone())?;

    // if approvals are enabled, create pending sale.
    if config.sale_approvals {
        return execute_create_pending_sale(deps, env, info, listing_id, listing, price, recipient);
    }

    // remove listing
    listings().remove(deps.storage, listing_id.clone())?;

    let buy_msg = asset_buy_msg(recipient, listing.token_id.clone());
    let asset_price = listing.asset_price.clone();
    let marketplace_fee = payment
        .amount
        .checked_sub(asset_price.amount)
        .map_err(|_| ContractError::InsuficientFunds {})?;
    let mut response = Response::new()
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
            funds: vec![asset_price],
        });

    if !marketplace_fee.is_zero() {
        response = response.add_message(BankMsg::Send {
            to_address: config.fee_recipient.to_string(),
            amount: vec![Coin {
                denom: payment.denom,
                amount: marketplace_fee,
            }],
        });
    }

    Ok(response)
}

fn execute_create_pending_sale(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    listing_id: String,
    listing: Listing,
    price: Coin,
    recipient: Addr,
) -> Result<Response, ContractError> {
    // query if there is a previous pending sale for this item
    let existing_pending_sale = pending_sales().idx.by_collection_and_token_id.item(
        deps.storage,
        (listing.collection.clone(), listing.token_id.clone()),
    );
    if let Ok(Some(_)) = existing_pending_sale {
        return Err(ContractError::PendingSaleAlreadyExists {
            collection: listing.collection.clone().to_string(),
            token_id: listing.token_id.clone(),
        });
    }

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
        recipient,
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
    let mut sub_msgs = vec![];

    // query if there is a listing in the asset contract (in case is out of sync)
    let asset_listing_resp = query_listing(&deps.querier, &listing.collection, &listing.token_id);
    // if there is a listing in the asset contract and has a previous reservation, unreserve it so we can reserve it again for approval queue
    if let Ok(asset_listing) = asset_listing_resp {
        if asset_listing.reserved.is_some() {
            let unreserve_msg = asset_unreserve_msg(listing.token_id.clone(), false);
            sub_msgs.push(WasmMsg::Execute {
                contract_addr: listing.collection.to_string(),
                msg: to_json_binary(&unreserve_msg)?,
                funds: vec![],
            });
        }
    }

    // Reserve the NFT in the asset contract
    let reserve_msg = asset_reserve_msg(
        listing.token_id.clone(),
        // marketplace contract should be the reserver
        env.contract.address.clone(),
        env.block.time.plus_seconds(86400),
    );
    sub_msgs.push(WasmMsg::Execute {
        contract_addr: listing.collection.to_string(),
        msg: to_json_binary(&reserve_msg)?,
        funds: vec![],
    });

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
        .add_messages(sub_msgs)
        .add_attribute("action", "pending_sale_created"))
}

pub fn execute_approve_sale(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pending_sale_id: String,
) -> Result<Response, ContractError> {
    // Only manager can approve
    only_manager(&info, &deps)?;

    let config = CONFIG.load(deps.storage)?;
    let pending_sale = pending_sales().load(deps.storage, pending_sale_id.clone())?;

    if env.block.time.seconds() >= pending_sale.expiration {
        return Err(ContractError::PendingSaleExpired {
            id: pending_sale_id,
        });
    }

    // Generate listing_id to find the listing
    let listing_id = generate_id(vec![
        pending_sale.collection.as_bytes(),
        pending_sale.token_id.as_bytes(),
    ]);
    let listing = listings().load(deps.storage, listing_id.clone())?;

    // Execute the buy on asset contract
    let buy_msg = asset_buy_msg(
        pending_sale.recipient.clone(),
        pending_sale.token_id.clone(),
    );

    // Use the asset_price stored on the listing to avoid fee changes affecting pending sales
    // Marketplace fee is the difference between the buyer price and the stored asset_price
    let asset_price = listing.asset_price;
    let marketplace_fee_amount = listing
        .price
        .amount
        .checked_sub(asset_price.amount)
        .map_err(|_| ContractError::InsuficientFunds {})?;

    // delete original listing
    listings().remove(deps.storage, listing_id.clone())?;

    // remove from queue
    pending_sales().remove(deps.storage, pending_sale_id.clone())?;

    let mut response = Response::new()
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
        // Send asset_price to asset contract (seller proceeds)
        .add_message(WasmMsg::Execute {
            contract_addr: pending_sale.collection.to_string(),
            msg: to_json_binary(&buy_msg)?,
            funds: vec![asset_price],
        });

    // Only send marketplace fee if it's greater than zero
    // CosmWasm doesn't allow sending empty coin amounts
    if !marketplace_fee_amount.is_zero() {
        response = response.add_message(BankMsg::Send {
            to_address: config.fee_recipient.to_string(),
            amount: vec![Coin {
                denom: pending_sale.price.denom,
                amount: marketplace_fee_amount,
            }],
        });
    }

    Ok(response)
}

fn remove_pending_sale(
    deps: DepsMut,
    pending_sale_id: String,
    pending_sale: PendingSale,
    reason: &str,
) -> Result<Response, ContractError> {
    let listing_id = generate_id(vec![
        pending_sale.collection.as_bytes(),
        pending_sale.token_id.as_bytes(),
    ]);

    // delete the listing
    listings().remove(deps.storage, listing_id)?;

    // query if there is a listing in the asset contract
    let asset_listing = query_listing(
        &deps.querier,
        &pending_sale.collection,
        &pending_sale.token_id,
    );

    let mut sub_msgs = vec![];

    // delist from asset contract only if it has a listing
    if asset_listing.is_ok() {
        let delist_msg = asset_delist_msg(pending_sale.token_id.clone());
        sub_msgs.push(WasmMsg::Execute {
            contract_addr: pending_sale.collection.to_string(),
            msg: to_json_binary(&delist_msg)?,
            funds: vec![],
        });
    }

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
            reason,
        ))
        .add_message(refund_msg)
        .add_messages(sub_msgs))
}

pub fn execute_reject_sale(
    deps: DepsMut,
    info: MessageInfo,
    pending_sale_id: String,
) -> Result<Response, ContractError> {
    only_manager(&info, &deps)?;

    let pending_sale = pending_sales().load(deps.storage, pending_sale_id.clone())?;

    remove_pending_sale(deps, pending_sale_id, pending_sale, "rejected_by_manager")
}

pub fn execute_reclaim_expired_sale(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pending_sale_id: String,
) -> Result<Response, ContractError> {
    let pending_sale = pending_sales().load(deps.storage, pending_sale_id.clone())?;

    if env.block.time.seconds() < pending_sale.expiration {
        return Err(ContractError::PendingSaleNotExpired {
            id: pending_sale_id,
        });
    }

    if info.sender != pending_sale.buyer {
        return Err(ContractError::Unauthorized {
            message: "only the buyer can reclaim an expired sale".to_string(),
        });
    }

    remove_pending_sale(deps, pending_sale_id, pending_sale, "expired")
}

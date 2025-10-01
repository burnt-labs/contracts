use std::env;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::Offer;
use crate::state::{init_auto_increment, next_auto_increment};
use crate::state::{offers, Config, CONFIG};
use blake2::{Blake2s256, Digest};
use cosmwasm_std::{
    ensure, entry_point, has_coins, to_json_binary, Addr, Binary, Coin, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdResult, WasmMsg,
};
use cw2::set_contract_version;
use cw721::Expiration;
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

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::CreateListing {
            collection,
            price,
            token_id,
        } => execute_create_listing(env, collection, price, token_id),
        ExecuteMsg::CancelListing {
            collection,
            token_id,
        } => execute_cancel_listing(collection, token_id),
        ExecuteMsg::PurchaseItem {
            collection,
            token_id,
            price,
        } => execute_purchase_item(deps, info, collection, token_id, price),
        ExecuteMsg::CreateOffer {
            collection,
            price,
            token_id,
        } => execute_create_offer(deps, info, api.addr_validate(&collection)?, price, token_id),
        ExecuteMsg::AcceptOffer { .. } => {
            Ok(Response::new().add_attribute("method", "accept_collection_offer"))
        }

        ExecuteMsg::CreateCollectionOffer { .. } => {
            Ok(Response::new().add_attribute("method", "create_collection_offer"))
        }
        ExecuteMsg::AcceptCollectionOffer { .. } => {
            Ok(Response::new().add_attribute("method", "accept_collection_offer"))
        }
    }
}

pub fn generate_id(parts: Vec<&[u8]>) -> String {
    let mut hasher = Blake2s256::new();
    for part in parts {
        hasher.update(part);
    }
    format!("{:x}", hasher.finalize())
}

pub fn execute_create_offer(
    deps: DepsMut,
    info: MessageInfo,
    collection: Addr,
    price: Coin,
    token_id: String,
) -> Result<Response, ContractError> {
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
    offers().update(deps.storage, id.to_string(), |prev| match prev {
        Some(_) => Err(ContractError::OfferAlreadyExists { id: id.to_string() }),
        None => Ok(offer),
    })?;

    Ok(Response::new().add_attribute("method", "create_offer"))
}
pub fn execute_create_listing(
    env: Env,
    collection: String,
    price: Coin,
    token_id: String,
) -> Result<Response, ContractError> {
    let list = asset::msg::AssetExtensionExecuteMsg::List {
        token_id,
        price,
        reservation: Some(asset::state::Reserve {
            reserver: env.contract.address,
            reserved_until: Expiration::AtTime(env.block.time.plus_seconds(3600)),
        }),
    };
    Ok(Response::new()
        .add_attribute("method", "create_listing")
        .add_message(WasmMsg::Execute {
            contract_addr: collection,
            msg: to_json_binary(&list)?,
            funds: vec![],
        }))
}

pub fn execute_cancel_listing(
    collection: String,
    token_id: String,
) -> Result<Response, ContractError> {
    let cancel_listing = asset::msg::AssetExtensionExecuteMsg::Delist { token_id };
    Ok(Response::new()
        .add_attribute("method", "cancel_listing")
        .add_message(WasmMsg::Execute {
            contract_addr: collection,
            msg: to_json_binary(&cancel_listing)?,
            funds: vec![],
        }))
}

pub fn execute_purchase_item(
    deps: DepsMut,
    info: MessageInfo,
    collection: String,
    token_id: String,
    price: Coin,
) -> Result<Response, ContractError> {
    let listing: asset::state::ListingInfo<Empty> = deps.querier.query_wasm_smart(
        collection.clone(),
        &asset::msg::AssetExtensionQueryMsg::GetListing {
            token_id: token_id.clone(),
        },
    )?;

    if listing.price != price {
        return Err(ContractError::InvalidPrice {
            expected: listing.price,
            actual: price,
        });
    }

    ensure!(
        has_coins(&info.funds, &price),
        ContractError::InvalidPrice {
            expected: listing.price,
            actual: price,
        }
    );
    let purchase_item = asset::msg::AssetExtensionExecuteMsg::Buy {
        token_id,
        recipient: Some(info.sender.to_string()),
    };
    Ok(Response::new()
        .add_attribute("method", "purchase_item")
        .add_message(WasmMsg::Execute {
            contract_addr: collection.clone(),
            msg: to_json_binary(&purchase_item)?,
            funds: info.funds,
        }))
}
#[entry_point]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(_deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Config<Addr>> {
    Ok(CONFIG.load(deps.storage)?)
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}

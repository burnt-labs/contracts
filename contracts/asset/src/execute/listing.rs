use cosmwasm_std::{Coin, CustomMsg, DepsMut, Env, MessageInfo, Response};
use cw721::traits::Cw721State;

use crate::{
    error::ContractError,
    msg::ReserveMsg,
    state::{AssetConfig, ListingInfo, Reserve},
};

use super::permissions::check_can_list;

pub fn list<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    price: Coin,
    reservation: Option<ReserveMsg>,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let mut response = Response::<TCustomResponseMsg>::default();
    let asset_config = AssetConfig::<TNftExtension>::default();
    // make sure the caller is the owner of the token
    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    // check if we can list the asset
    check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;
    // make sure the price is greater than zero
    if price.amount.is_zero() {
        return Err(ContractError::InvalidListingPrice {
            price: price.amount.u128(),
        });
    }

    // Ensure the listing does not already exist
    let old_listing = asset_config.listings.may_load(deps.storage, &id)?;
    if old_listing.is_some() {
        return Err(ContractError::ListingAlreadyExists { id });
    }
    let reservation = match reservation {
        Some(reserve_msg) => {
            if reserve_msg.reserved_until <= env.block.time {
                return Err(ContractError::InvalidReservationExpiration {
                    reserved_until: reserve_msg.reserved_until.seconds(),
                });
            }
            Some(Reserve {
                reserver: if let Some(reserver) = reserve_msg.reserver {
                    deps.api.addr_validate(&reserver)?
                } else {
                    info.sender.clone()
                },
                reserved_until: reserve_msg.reserved_until,
            })
        }
        None => None,
    };
    // Save the listing
    let listing = ListingInfo {
        id: id.clone(),
        seller: nft_info.owner.clone(),
        price: price.clone(),
        reserved: reservation.clone(),
    };
    asset_config.listings.save(deps.storage, &id, &listing)?;
    response = response
        .add_attribute("action", "list")
        .add_attribute("id", id)
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("price", price.amount.to_string())
        .add_attribute("denom", price.denom.to_string())
        .add_attribute("seller", nft_info.owner.clone().to_string())
        .add_attribute(
            "reserved_until",
            reservation.map_or("none".to_string(), |r| r.reserved_until.to_string()),
        );
    Ok(response)
}

pub fn delist<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or_else(|| ContractError::ListingNotFound { id: id.clone() })?;

    // only the ones who can list can delist
    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;
    if listing.seller != nft_info.owner {
        return Err(ContractError::StaleListing {});
    }

    asset_config.listings.remove(deps.storage, &id)?;

    Ok(Response::default()
        .add_attribute("action", "delist")
        .add_attribute("id", listing.id)
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("seller", listing.seller.to_string()))
}

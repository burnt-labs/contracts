use cosmwasm_std::{CustomMsg, DepsMut, Env, MessageInfo, Response};
use cw721::{Expiration, traits::Cw721State};

use crate::{
    error::ContractError,
    msg::ReserveMsg,
    state::{AssetConfig, Reserve},
};

use super::permissions::check_can_list;

pub fn reserve<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    reservation: ReserveMsg,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let mut listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or_else(|| ContractError::ListingNotFound { id: id.clone() })?;

    // only the ones who can list can reserve
    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;
    if listing.seller != nft_info.owner {
        return Err(ContractError::StaleListing {});
    }

    if reservation.reserved_until <= env.block.time {
        return Err(ContractError::InvalidReservationExpiration {
            reserved_until: reservation.reserved_until.seconds(),
        });
    }

    if let Some(reserved) = &listing.reserved {
        if !Expiration::AtTime(reserved.reserved_until).is_expired(&env.block) {
            return Err(ContractError::ReservedAsset { id: id.clone() });
        }
    }

    let reserver = if let Some(reserver) = reservation.reserver {
        deps.api.addr_validate(&reserver)?
    } else {
        info.sender.clone()
    };
    listing.reserved = Some(Reserve {
        reserver: reserver.clone(),
        reserved_until: reservation.reserved_until,
    });
    asset_config.listings.save(deps.storage, &id, &listing)?;

    Ok(Response::default()
        .add_attribute("action", "reserve")
        .add_attribute("id", id)
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("reserver", reserver.to_string())
        .add_attribute("reserved_until", reservation.reserved_until.to_string()))
}

pub fn unreserve<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    delist: bool,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let mut listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or_else(|| ContractError::ListingNotFound { id: id.clone() })?;

    let reserved = listing
        .reserved
        .as_ref()
        .ok_or_else(|| ContractError::ReservationNotFound { id: id.clone() })?;

    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    if listing.seller != nft_info.owner {
        return Err(ContractError::StaleListing {});
    }

    if reserved.reserver != info.sender {
        check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;
    }

    let response = Response::<TCustomResponseMsg>::default()
        .add_attribute("action", "unreserve")
        .add_attribute("id", id.clone())
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("reserver", info.sender.to_string());

    if delist {
        asset_config.listings.remove(deps.storage, &id)?;
        return Ok(response.add_attribute("delisted", "true"));
    }

    listing.reserved = None;
    asset_config.listings.save(deps.storage, &id, &listing)?;

    Ok(response.add_attribute("delisted", "false"))
}

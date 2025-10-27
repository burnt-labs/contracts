use cosmwasm_std::{Coin, CustomMsg, DepsMut, Env, MessageInfo, Response};
use cw721::traits::Cw721State;

use crate::{
    error::ContractError, msg::ReserveMsg, state::{AssetConfig, ListingInfo, Reserve}
};

use super::permissions::check_can_list;

pub fn list<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    price: Coin,
    reservation: Option<ReserveMsg>,
    marketplace_fee_bps: Option<u16>,
    marketplace_fee_recipient: Option<String>,
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

    let (validated_marketplace_fee_bps, validated_marketplace_fee_recipient) =
        match (marketplace_fee_bps, marketplace_fee_recipient) {
            (Some(bps), Some(recipient)) => {
                if bps > 10_000 {
                    return Err(ContractError::InvalidMarketplaceFee { bps, recipient });
                }
                let recipient_addr = deps.api.addr_validate(&recipient)?;
                response = response.add_attribute("marketplace_fee_bps", bps.to_string()).add_attribute("marketplace_fee_recipient", recipient_addr.to_string());
                (Some(bps), Some(recipient_addr))
            }
            (Some(bps), None) => {
                let recipient_addr = info.sender.clone();
                if bps > 10_000 {
                    return Err(ContractError::InvalidMarketplaceFee { bps, recipient: recipient_addr.to_string() });
                }
                response = response.add_attribute("marketplace_fee_bps", bps.to_string()).add_attribute("marketplace_fee_recipient", recipient_addr.to_string());
                (Some(bps), Some(recipient_addr))
            }
            (None, Some(recipient)) => {
                return Err(ContractError::InvalidMarketplaceFee { bps: 0, recipient });
            }
            (None, None) => (None, None),
        };
    // Ensure the listing does not already exist
    let old_listing = asset_config.listings.may_load(deps.storage, &id)?;
    if old_listing.is_some() {
        return Err(ContractError::ListingAlreadyExists { id });
    }
    // todo convert reservation msg to state reservation
    let reservation = match reservation {
        Some(reserve_msg) => Some(Reserve {
            reserver: if let Some(reserver) = reserve_msg.reserver { deps.api.addr_validate(&reserver)? } else { info.sender.clone() },
            reserved_until: reserve_msg.reserved_until,
        }),
        None => None,
    };
    // Save the listing
    let listing = ListingInfo {
        id: id.clone(),
        seller: nft_info.owner.clone(),
        price: price.clone(),
        reserved: reservation.clone(),
        marketplace_fee_bps: validated_marketplace_fee_bps,
        marketplace_fee_recipient: validated_marketplace_fee_recipient.clone(),
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

    asset_config.listings.remove(deps.storage, &id)?;

    Ok(Response::default()
        .add_attribute("action", "delist")
        .add_attribute("id", listing.id)
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("seller", listing.seller.to_string()))
}

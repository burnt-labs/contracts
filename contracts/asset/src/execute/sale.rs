use cosmwasm_std::{BankMsg, Coin, CustomMsg, DepsMut, Env, MessageInfo, Response};
use cw721::{Expiration, traits::Cw721State};

use crate::{error::ContractError, state::AssetConfig};

pub fn buy<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    recipient: Option<String>,
    deductions: Vec<(String, Coin, String)>,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let mut listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or(ContractError::ListingNotFound { id: id.clone() })?;

    let mut nft_info = asset_config
        .cw721_config
        .nft_info
        .load(deps.storage, &id)
        .map_err(|_| ContractError::ListingNotFound { id: id.clone() })?;

    let price = listing.price.clone();
    let seller = listing.seller.clone();

    if seller != nft_info.owner {
        return Err(ContractError::StaleListing {});
    }

    // only one coin can be sent
    if info.funds.len() > 1 {
        return Err(ContractError::MultiplePaymentsSent {});
    }

    let mut payment = info
        .funds
        .iter()
        .find(|coin| coin.denom == price.denom)
        .ok_or_else(|| ContractError::NoPayment {})?
        .clone();

    // check for underpayment but overpayment are absorbed if an exact price
    // plugin is not set on the asset
    if payment.amount.lt(&price.amount) || payment.denom != price.denom {
        return Err(ContractError::InvalidPayment {
            price: payment.amount.u128(),
            denom: payment.denom.clone(),
        });
    }

    let mut response = Response::<TCustomResponseMsg>::default();

    // remove all other deductions e.g. royalties from payment
    for (_, amount, _) in deductions {
        payment.amount = payment
            .amount
            .checked_sub(amount.amount)
            .map_err(|_| ContractError::InsufficientFunds {})?;
    }

    let buyer = match recipient {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => info.sender.clone(),
    };

    if let Some(reserved) = listing.reserved {
        if Expiration::AtTime(reserved.reserved_until).is_expired(&env.block) {
            listing.reserved = None;
        } else if reserved.reserver != info.sender {
            // if the reserver is not the sender, return an error
            return Err(ContractError::Unauthorized {});
        }
    }

    nft_info.owner = buyer.clone();
    nft_info.approvals.clear();
    asset_config
        .cw721_config
        .nft_info
        .save(deps.storage, &id, &nft_info)?;

    asset_config.listings.remove(deps.storage, &id)?;

    response = response
        .add_message(BankMsg::Send {
            to_address: seller.to_string(),
            amount: vec![payment.clone()], // we send the remaining payment after deductions to the seller
        })
        .add_attribute("action", "buy")
        .add_attribute("id", id)
        .add_attribute("price", price.amount.to_string())
        .add_attribute("denom", price.denom)
        .add_attribute("seller", seller.to_string())
        .add_attribute("buyer", buyer.to_string());
    Ok(response)
}

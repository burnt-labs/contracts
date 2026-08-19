/// These are default plugins that can be used out of the box.
use cosmwasm_std::{Attribute, BankMsg, Coin, CosmosMsg, CustomMsg, Empty, StdResult, SubMsg};
use cw721::Expiration;

use crate::plugin::{DefaultXionAssetContext, PluginCtx};

/// opinionated plugin functions for some common actions
/// e.g. listing an asset, delisting an asset, transferring an asset, buying an asset
/// we only need to check things not already covered by the core logic
/// this plugin checks that the price of purchase matches the ask price exactly
/// if an ask price is set
pub fn exact_price_plugin(ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>) -> StdResult<bool> {
    // check if the exact price is met
    if let Some(ask_price) = &ctx.data.ask_price {
        if let Some(funds) = ctx.info.funds.iter().find(|c| c.denom == ask_price.denom) {
            if funds.amount.u128() != ask_price.amount.u128() {
                return Err(cosmwasm_std::StdError::generic_err(format!(
                    "Exact price not met: {} required, {} provided",
                    ask_price.amount.u128(),
                    funds.amount.u128()
                )));
            }
        } else {
            return Err(cosmwasm_std::StdError::generic_err(
                "Exact price not met: no funds provided".to_string(),
            ));
        }
    }
    Ok(true)
}

/// this plugin checks that the price of listing is above the minimum price
/// if a minimum price is set
pub fn min_price_plugin(ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>) -> StdResult<bool> {
    // remove royalty and marketplace fees from the ask price if set
    if let Some(royalty) = &ctx.royalty.collection_royalty_bps {
        ctx.data.ask_price = ctx.data.ask_price.as_mut().map(|p| {
            let royalty_amount = p.amount.multiply_ratio(*royalty as u128, 10_000u128);
            Coin {
                denom: p.denom.clone(),
                amount: p.amount.checked_sub(royalty_amount).unwrap_or_default(),
            }
        });
    }
    if let Some(marketplace_fee) = &ctx.data.marketplace_fee_bps {
        ctx.data.ask_price = ctx.data.ask_price.as_mut().map(|p| {
            let marketplace_fee_amount = p
                .amount
                .multiply_ratio(*marketplace_fee as u128, 10_000u128);
            Coin {
                denom: p.denom.clone(),
                amount: p
                    .amount
                    .checked_sub(marketplace_fee_amount)
                    .unwrap_or_default(),
            }
        });
    }
    // check if the minimum price is met
    if let Some(min_price) = &ctx.data.min_price {
        if let Some(ask_price) = ctx.data.ask_price.clone() {
            if ask_price.denom != min_price.denom {
                return Err(cosmwasm_std::StdError::generic_err(format!(
                    "ask price denom {} does not match minimum price denom {}",
                    ask_price.denom, min_price.denom
                )));
            }
            if ask_price.amount.u128() < min_price.amount.u128() {
                return Err(cosmwasm_std::StdError::generic_err(format!(
                    "Minimum price not met: {} required, {} provided",
                    min_price, ask_price
                )));
            }
        } else {
            return Err(cosmwasm_std::StdError::generic_err(
                "Minimum price not met: no price provided".to_string(),
            ));
        }
    }
    Ok(true)
}

/// this plugin checks that the current time is after the not_before time
/// if a not_before time is set
pub fn not_before_plugin(ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>) -> StdResult<bool> {
    if !ctx.data.not_before.is_expired(&ctx.env.block) {
        return Err(cosmwasm_std::StdError::generic_err(format!(
            "Current time {} is before the allowed listing time {}",
            ctx.env.block.time, ctx.data.not_before
        )));
    }

    Ok(true)
}

/// this plugin checks that the current time is before the not_after time
/// if a not_after time is set
pub fn not_after_plugin(ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>) -> StdResult<bool> {
    if ctx.data.not_after.is_expired(&ctx.env.block) {
        return Err(cosmwasm_std::StdError::generic_err(format!(
            "Current time {} is after the allowed listing time {}",
            ctx.env.block.time, ctx.data.not_after
        )));
    }
    Ok(true)
}

// this plugin checks that every reservation does not exceed the time lock
// if a time lock is set
// e.g. if the collection has a time lock of 1 week, no reservation
// can be made that exceeds 1 week from now
// this is to prevent an indefinite reservation that exceeds the time lock
pub fn time_lock_plugin(ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>) -> StdResult<bool> {
    if let Some(time_lock) = &ctx.data.time_lock {
        if let Some(reservation) = &ctx.data.reservation {
            if Expiration::AtTime(reservation.reserved_until).gt(&Expiration::AtTime(
                ctx.env.block.time.plus_seconds(time_lock.as_secs()),
            )) {
                return Err(cosmwasm_std::StdError::generic_err(format!(
                    "Reservation end time {} exceeds the collection time lock {}",
                    reservation.reserved_until,
                    Expiration::AtTime(ctx.env.block.time.plus_seconds(time_lock.as_secs()))
                )));
            }
        }
    }
    Ok(true)
}

pub fn royalty_plugin(ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>) -> StdResult<bool> {
    let (recipient, bps) = match (
        ctx.royalty.collection_royalty_recipient.clone(),
        ctx.royalty.collection_royalty_bps,
    ) {
        (Some(recipient), Some(bps)) => (recipient, bps),
        _ => return Ok(true),
    };

    if bps == 0 {
        return Ok(true);
    }

    if let Some(ask_price) = &ctx.data.ask_price {
        if ask_price.amount.is_zero() {
            return Err(cosmwasm_std::StdError::generic_err(
                "Ask price is zero, cannot calculate royalty".to_string(),
            ));
        }
    } else {
        Err(cosmwasm_std::StdError::generic_err(
            "No ask price set for royalty calculation".to_string(),
        ))?;
    }
    let fund = ctx
        .info
        .funds
        .iter()
        .find(|c| c.denom == ctx.data.ask_price.as_ref().unwrap().denom);
    if fund.is_none() {
        Err(cosmwasm_std::StdError::generic_err(
            "No funds provided for royalty".to_string(),
        ))?;
    }
    let fund = fund.unwrap();
    let royalty_amount = fund.amount.multiply_ratio(bps as u128, 10_000u128);
    if royalty_amount.is_zero() {
        return Ok(true);
    }

    let royalty_coin = Coin {
        denom: fund.denom.clone(),
        amount: royalty_amount,
    };

    ctx.response.attributes.push(Attribute {
        key: "royalty_amount".to_string(),
        value: royalty_coin.clone().to_string(),
    });
    ctx.response.attributes.push(Attribute {
        key: "royalty_recipient".to_string(),
        value: recipient.to_string(),
    });

    let msg = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
        to_address: recipient.to_string(),
        amount: vec![royalty_coin.clone()],
    }));

    ctx.response.messages.push(msg);
    ctx.deductions
        .push((recipient.to_string(), royalty_coin, "royalty".to_string()));

    Ok(true)
}

pub fn allowed_marketplaces_plugin(
    ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
) -> StdResult<bool> {
    if let Some(allowed) = &ctx.data.allowed_marketplaces {
        if allowed.is_empty() {
            return Ok(true);
        }

        let buyer = ctx
            .data
            .buyer
            .clone()
            .unwrap_or_else(|| ctx.info.sender.clone());

        if !allowed.contains(&buyer) {
            return Err(cosmwasm_std::StdError::generic_err(
                "buyer is not an allowed marketplace",
            ));
        }
    }

    Ok(true)
}

pub fn allowed_currencies_plugin(
    ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
) -> StdResult<bool> {
    let allowed = match ctx.data.allowed_currencies.as_ref() {
        Some(denoms) if !denoms.is_empty() => denoms,
        _ => return Ok(true),
    };

    use std::collections::HashSet;

    let allowed_set: HashSet<&str> = allowed.iter().map(|d| d.denom.as_str()).collect();

    if let Some(price) = &ctx.data.ask_price {
        if !allowed_set.contains(price.denom.as_str()) {
            return Err(cosmwasm_std::StdError::generic_err(
                "ask price currency is not allowed",
            ));
        }
    }

    if let Some(min_price) = &ctx.data.min_price {
        if !allowed_set.contains(min_price.denom.as_str()) {
            return Err(cosmwasm_std::StdError::generic_err(
                "minimum price currency is not allowed",
            ));
        }
    }

    for coin in &ctx.info.funds {
        if !allowed_set.contains(coin.denom.as_str()) {
            return Err(cosmwasm_std::StdError::generic_err(format!(
                "currency {} is not allowed",
                coin.denom
            )));
        }
    }
    Ok(true)
}
/// This plugin checks that raw transfers are enabled. If royalty info is set,
/// transfers are disabled and all transfers must go through the buy flow.
pub fn is_transfer_enabled_plugin<T, U: CustomMsg>(ctx: &mut PluginCtx<T, U>) -> StdResult<bool> {
    if ctx.royalty.collection_royalty_bps.is_some()
        && ctx.royalty.collection_royalty_recipient.is_some()
    {
        return Err(cosmwasm_std::StdError::generic_err(
            "raw transfers are disabled when royalty info is set",
        ));
    }

    Ok(true)
}

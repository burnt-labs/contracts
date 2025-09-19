use cosmwasm_std::{Addr, CustomMsg, DepsMut, Empty, Env, MessageInfo, Response, StdResult};
use cw721::msg::Cw721ExecuteMsg;

/// Shared context passed through the pipeline, mutated by plugins.
pub struct PluginCtx<'a, Context, TCustomResponseMsg>
where
    TCustomResponseMsg: CustomMsg,
{
    pub deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
    pub funds_total: u128,                     // sum(info.funds)
    pub funds_remaining: u128,                 // decreases as plugins deduct
    pub deductions: Vec<(String, u128, Addr)>, // (reason, amount, to)

    /// The response being built up by the plugins.
    pub response: Response<TCustomResponseMsg>,

    pub data: Context,
}

pub struct DefaultXionAssetContext {
    pub token_id: String,
    pub seller: Addr,
    pub buyer: Option<Addr>,

    pub min_price: Option<u128>, // minimum price an asset can be listed for
    pub ask_price: Option<u128>, // if (List) or None on transfer

    pub not_before: Option<u64>, // timestamp before which an asset cannot be listed
    pub not_after: Option<u64>,  // timestamp after which an asset cannot be listed

    pub collection_royalty_bps: Option<u16>,
    pub collection_royalty_recipient: Option<Addr>,
    pub collection_royalty_on_primary: Option<bool>,

    pub nft_royalty_bps: Option<u16>,
    pub nft_royalty_recipient: Option<Addr>,

    pub primary_complete: bool,
}

pub type DefaultPluginCtx<'a> = PluginCtx<'a, DefaultXionAssetContext, Empty>;
/// The concept of a plugin is to be able to hook into the execution flow when a certain action
/// is performed.
/// e.g. when an asset is listed, delisted, transferred, or sold.
/// Plugins can modify the context, and return errors to abort the action.
/// Plugins can also add custom messages to be executed after the main action is performed.
/// This returned response is merged into the main response.
/// We have a default implementation that does nothing for convenience.
/// Bare in mind that the context should be shared between plugins, so they can affect each other.
pub trait Plugin<Context, TCustomResponseMsg, TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg>
where
    TCustomResponseMsg: CustomMsg,
{
    fn run_plugin(
        &self,
        _msg: Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg>,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<Response<TCustomResponseMsg>> {
        Ok(Response::new())
        // TODO check the msg and run the appropriate plugin logic
        // e.g. if msg is List, run the list plugin
        // if msg is Delist, run the delist plugin
    }
}

/// These are default plugins that can be used out of the box.
pub mod default_plugins {
    use cosmwasm_std::{Deps, Empty};
    use cw721::{error::Cw721ContractError, state::NftInfo, traits::Cw721State};

    use crate::{error::ContractError, state::AssetConfig};

    use super::*;

    /// opinionated plugin functions for some common actions
    /// e.g. listing an asset, delisting an asset, transferring an asset, buying an asset
    /// we only need to check things not already covered by the core logic
    pub fn list_plugin(
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
        nft_info: &NftInfo<Empty>,
    ) -> StdResult<Response<Empty>> {
        // check if the listing price is above the minimum price
        if let Some(min_price) = ctx.data.min_price {
            if let Some(ask_price) = ctx.data.ask_price {
                if ask_price < min_price {
                    return Err(cosmwasm_std::StdError::generic_err(format!(
                        "Listing price {} is below the minimum price {}",
                        ask_price, min_price
                    )));
                } else {
                    // confirm the listing price is deducted from the funds remaining
                    if ctx.funds_remaining < ask_price {
                        return Err(cosmwasm_std::StdError::generic_err(format!(
                            "Insufficient funds: {} remaining, {} required",
                            ctx.funds_remaining, ask_price
                        )));
                    } else {
                        // deduct the listing price from the funds remaining
                        ctx.funds_remaining -= ask_price;
                        // add a deduction entry
                        ctx.deductions.push((
                            "listing_price".to_string(),
                            ask_price,
                            ctx.data.seller.clone(),
                        ));
                    }
                }
            } else {
                return Err(cosmwasm_std::StdError::generic_err(
                    "Listing price is not set".to_string(),
                ));
            }
        }
        // check if the current time is within the allowed listing window
        let current_time = ctx.env.block.time.seconds();
        if let Some(not_before) = ctx.data.not_before {
            if current_time < not_before {
                return Err(cosmwasm_std::StdError::generic_err(format!(
                    "Current time {} is before the allowed listing time {}",
                    current_time, not_before
                )));
            }
        }
        if let Some(not_after) = ctx.data.not_after {
            if current_time > not_after {
                return Err(cosmwasm_std::StdError::generic_err(format!(
                    "Current time {} is after the allowed listing time {}",
                    current_time, not_after
                )));
            }
        }
        Ok(Response::new())
    }
}

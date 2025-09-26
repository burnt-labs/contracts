use std::time::Duration;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, BankMsg, Binary, Coin, CosmosMsg, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Response, StdResult, SubMsg,
};
use cw721::{
    Expiration,
    error::Cw721ContractError,
    msg::Cw721ExecuteMsg,
    traits::{
        Cw721CustomMsg, Cw721Execute, Cw721State, FromAttributesState, StateFactory,
        ToAttributesState,
    },
};

use crate::{
    error::ContractError,
    msg::AssetExtensionExecuteMsg,
    state::{AssetConfig, Reserve},
    traits::{AssetContract, DefaultAssetContract},
};

/// Shared context passed through the pipeline, mutated by plugins.
pub struct PluginCtx<'a, Context, TCustomResponseMsg>
where
    TCustomResponseMsg: CustomMsg,
{
    pub deps: Deps<'a>,
    pub env: Env,
    pub info: MessageInfo,

    /// The response being built up by the plugins.
    pub response: Response<TCustomResponseMsg>,

    pub data: Context,
}

pub struct DefaultXionAssetContext {
    pub token_id: String,
    pub seller: Option<Addr>,
    pub buyer: Option<Addr>,

    pub min_price: Option<Coin>, // minimum price an asset can be listed for
    pub ask_price: Option<Coin>, // if (List) or None on transfer

    pub not_before: Expiration, // timestamp before which an asset cannot be listed
    pub not_after: Expiration,  // timestamp after which an asset cannot be listed
    pub reservation: Option<Reserve>,
    pub time_lock: Option<Duration>,

    pub collection_royalty_bps: Option<u16>,
    pub collection_royalty_recipient: Option<Addr>,
    pub collection_royalty_on_primary: Option<bool>,

    pub nft_royalty_bps: Option<u16>,
    pub nft_royalty_recipient: Option<Addr>,
    pub nft_royalty_on_primary: Option<bool>,

    pub primary_complete: bool,

    pub allowed_marketplaces: Option<Vec<Addr>>,
    pub allowed_currencies: Option<Vec<Coin>>,
}

impl Default for DefaultXionAssetContext {
    fn default() -> Self {
        DefaultXionAssetContext {
            token_id: "".to_string(),
            seller: None,
            buyer: None,
            min_price: None,
            ask_price: None,
            not_before: Expiration::Never {},
            not_after: Expiration::Never {},
            reservation: None,
            collection_royalty_bps: None,
            collection_royalty_recipient: None,
            collection_royalty_on_primary: None,
            nft_royalty_bps: None,
            nft_royalty_recipient: None,
            nft_royalty_on_primary: None,
            primary_complete: false,
            allowed_marketplaces: None,
            allowed_currencies: None,
            time_lock: None,
        }
    }
}

pub type DefaultPluginCtx<'a> = PluginCtx<'a, DefaultXionAssetContext, Empty>;

#[cw_serde]
pub enum Plugin {
    ExactPrice {
        amount: Coin,
    },
    MinimumPrice {
        amount: Coin,
    },
    RequiresProof {
        proof: Vec<u8>,
    },
    NotBefore {
        time: Expiration,
    },
    NotAfter {
        time: Expiration,
    },
    TimeLock {
        time: Duration,
    },
    Royalty {
        bps: u16,
        recipient: Addr,
        on_primary: bool,
    },
    AllowedMarketplaces {
        marketplaces: Vec<Addr>,
    },
    AllowedCurrencies {
        denoms: Vec<Coin>,
    },
}

impl Plugin {
    // TODO
    // break this out into functions for each plugin type
    // so we can call them individually if needed
    pub fn run_asset_plugin(
        &self,
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
        match self {
            Plugin::ExactPrice { amount } => {
                ctx.data.ask_price = Some(amount.clone());
                default_plugins::exact_price_plugin(ctx)?;
            }
            Plugin::MinimumPrice { amount } => {
                ctx.data.min_price = Some(amount.clone());
                default_plugins::min_price_plugin(ctx)?;
            }
            Plugin::RequiresProof { .. } => {}
            Plugin::NotBefore { time } => {
                ctx.data.not_before = *time;
                default_plugins::not_before_plugin(ctx)?;
            }
            Plugin::NotAfter { time } => {
                ctx.data.not_after = *time;
                default_plugins::not_after_plugin(ctx)?;
            }
            Plugin::Royalty {
                bps,
                recipient,
                on_primary,
            } => {
                ctx.data.collection_royalty_bps = Some(*bps);
                ctx.data.collection_royalty_recipient = Some((*recipient).clone());
                ctx.data.collection_royalty_on_primary = Some(*on_primary);
                default_plugins::royalty_plugin(ctx)?;
            }
            Plugin::AllowedMarketplaces { marketplaces } => {
                ctx.data.allowed_marketplaces = Some(marketplaces.clone());
                default_plugins::allowed_marketplaces_plugin(ctx)?;
            }
            Plugin::AllowedCurrencies { denoms } => {
                ctx.data.allowed_currencies = Some(denoms.clone());
                default_plugins::allowed_currencies_plugin(ctx)?;
            }
            Plugin::TimeLock { time } => {
                ctx.data.time_lock = Some(*time);
                default_plugins::time_lock_plugin(ctx)?;
            }
        }
        Ok(true)
    }
}

/// The concept of a plugin is to be able to hook into the execution flow when a certain action
/// is performed.
/// e.g. when an asset is listed, de-listed, transferred, or sold.
/// Plugins can modify their context, and return errors to abort the action.
/// Plugins can also add custom messages to be executed after the main action is performed.
/// This returned response is merged into the main response.
/// We have a default implementation that does nothing for convenience.
/// Bare in mind that the context is shared between all plugins(code) that run so they can affect each other.
/// This trait is expected to be implemented by an asset contract or any contract conforming to cw721 standard.
pub trait PluggableAsset<
    Context,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TExtensionMsg,
    TCustomResponseMsg,
> where
    TCollectionExtension: Cw721State,
    TCollectionExtension: FromAttributesState + ToAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg,
    TCollectionExtensionMsg: StateFactory<TCollectionExtension>,
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg,
    TNftExtensionMsg: StateFactory<TNftExtension>,
    TCustomResponseMsg: CustomMsg,
    Self: Cw721Execute<
            TNftExtension,
            TNftExtensionMsg,
            TCollectionExtension,
            TCollectionExtensionMsg,
            TExtensionMsg,
            TCustomResponseMsg,
        >,
{
    /// Use this method instead of the execute method of the cw721 contract to
    /// execute plugins before executing the main action.
    /// After plugins have executed, the execute method of cw721 is called.
    fn execute_pluggable(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg>,
    ) -> Result<Response<TCustomResponseMsg>, Cw721ContractError> {
        let plugin_response: Response<TCustomResponseMsg>;
        {
            let mut plugin_ctx = Self::get_plugin_ctx(deps.as_ref(), env, info);

            match &msg {
                Cw721ExecuteMsg::TransferNft {
                    recipient,
                    token_id,
                } => self.on_transfer_plugin(recipient, token_id, &mut plugin_ctx)?,
                Cw721ExecuteMsg::UpdateExtension { msg } => {
                    self.on_update_extension_plugin(&msg, &mut plugin_ctx)?
                }
                _ => true,
            };
            plugin_response = plugin_ctx.response;
        }
        let mut response = self.execute(deps, env, info, msg)?;

        response.messages.extend(plugin_response.messages);
        response.events.extend(plugin_response.events);
        response.attributes.extend(plugin_response.attributes);

        if let Some(plugin_data) = plugin_response.data {
            match &mut response.data {
                Some(existing) => {
                    let mut combined = Vec::with_capacity(existing.len() + plugin_data.len());
                    combined.extend_from_slice(existing.as_slice());
                    combined.extend_from_slice(plugin_data.as_slice());
                    *existing = Binary::from(combined);
                }
                None => response.data = Some(plugin_data),
            }
        }

        Ok(response)
    }

    fn on_transfer_plugin(
        &self,
        _recipient: &String,
        _token_id: &String,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_update_extension_plugin<'a>(
        &self,
        _msg: &TExtensionMsg,
        _ctx: &'a mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_list_plugin<'a>(
        &self,
        _token_id: &String,
        _price: &Coin,
        _reservation: &Option<Reserve>,
        _ctx: &'a mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_delist_plugin(
        &self,
        _token_id: &String,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_buy_plugin(
        &self,
        _token_id: &String,
        _recipient: &Option<String>,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_reserve_plugin(
        &self,
        _token_id: &String,
        _reserver: &Reserve,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn get_plugin_ctx<'a>(
        deps: Deps<'a>,
        env: &Env,
        info: &MessageInfo,
    ) -> PluginCtx<'a, Context, TCustomResponseMsg>;
}

impl<TNftExtension, TNftExtensionMsg, TCollectionExtension, TCollectionExtensionMsg>
    PluggableAsset<
        DefaultXionAssetContext,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        AssetExtensionExecuteMsg,
        Empty,
    >
    for DefaultAssetContract<
        '_,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
    >
where
    TCollectionExtension: Cw721State + FromAttributesState + ToAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension> + Default,
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg,
    TNftExtensionMsg: StateFactory<TNftExtension>,
{
    fn on_transfer_plugin(
        &self,
        recipient: &String,
        token_id: &String,
        ctx: &mut DefaultPluginCtx,
    ) -> StdResult<bool> {
        // for transfers we run the royalty plugin if set
        let royalty_plugin = AssetConfig::<TNftExtension>::default()
            .collection_plugins
            .may_load(ctx.deps.storage, "Royalty")?;
        ctx.data.token_id = token_id.to_string();
        ctx.data.buyer = Some(ctx.deps.api.addr_validate(&recipient)?);
        ctx.data.seller = Some(ctx.info.sender.clone());
        if let Some(plugin) = royalty_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        Ok(true)
    }

    fn on_update_extension_plugin<'a>(
        &self,
        msg: &AssetExtensionExecuteMsg,
        ctx: &mut DefaultPluginCtx,
    ) -> StdResult<bool> {
        match msg {
            AssetExtensionExecuteMsg::List {
                token_id,
                price,
                reservation,
            } => self.on_list_plugin(token_id, price, reservation, ctx),
            AssetExtensionExecuteMsg::Reserve {
                token_id,
                reservation,
            } => self.on_reserve_plugin(token_id, reservation, ctx),
            AssetExtensionExecuteMsg::Delist { token_id } => self.on_delist_plugin(token_id, ctx),
            AssetExtensionExecuteMsg::Buy {
                token_id,
                recipient,
            } => self.on_buy_plugin(token_id, recipient, ctx),
        }
    }

    fn on_list_plugin(
        &self,
        token_id: &String,
        price: &Coin,
        reservation: &Option<Reserve>,
        ctx: &mut DefaultPluginCtx,
    ) -> StdResult<bool> {
        // for listings we run the minimum price, not before, not after plugins if set
        let config = AssetConfig::<TNftExtension>::default();
        let min_price_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "MinimumPrice")?;
        let not_before_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "NotBefore")?;
        let not_after_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "NotAfter")?;
        let allowed_currencies_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "AllowedCurrencies")?;
        ctx.data.token_id = token_id.to_string();
        ctx.data.ask_price = Some(price.clone());
        ctx.data.reservation = reservation.clone();
        ctx.data.seller = Some(ctx.info.sender.clone());
        if let Some(plugin) = min_price_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        if let Some(plugin) = not_before_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        if let Some(plugin) = not_after_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        if let Some(plugin) = allowed_currencies_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        Ok(true)
    }

    fn on_delist_plugin(&self, _token_id: &String, _ctx: &mut DefaultPluginCtx) -> StdResult<bool> {
        Ok(true)
    }

    fn on_buy_plugin(
        &self,
        token_id: &String,
        _recipient: &Option<String>,
        ctx: &mut DefaultPluginCtx,
    ) -> StdResult<bool> {
        // for buys we run the exact price, then allowed marketplaces and royalty plugins if set
        let config = AssetConfig::<TNftExtension>::default();
        let allowed_marketplaces_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "AllowedMarketplaces")?;
        let allowed_currencies_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "AllowedCurrencies")?;
        let royalty_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "Royalty")?;
        ctx.data.token_id = token_id.to_string();
        ctx.data.buyer = Some(ctx.info.sender.clone());
        // we need to get the listing info to get the ask price
        let listing = self
            .config
            .listings
            .load(ctx.deps.storage, token_id.as_str())
            .map_err(|_| ContractError::ListingNotFound {
                id: token_id.clone(),
            })?;
        ctx.data.ask_price = Some(listing.price.clone());
        if let Some(plugin) = allowed_currencies_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        // Exact price plugin disabled for buys for now
        // if let Some(plugin) = exact_price_plugin {
        //     plugin.run_asset_plugin(ctx)?;
        // }
        if let Some(plugin) = allowed_marketplaces_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        if let Some(plugin) = royalty_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        Ok(true)
    }

    fn on_reserve_plugin(
        &self,
        token_id: &String,
        reservation: &Reserve,
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
        let config = AssetConfig::<TNftExtension>::default();
        // we run the allowed marketplaces and not_after plugin if set
        ctx.data.buyer = ctx.info.sender.clone().into();
        ctx.data.reservation = Some(reservation.clone());
        ctx.data.token_id = token_id.to_string();
        let allowed_marketplaces_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "AllowedMarketplaces")?;
        let time_lock_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "TimeLock")?;
        if let Some(plugin) = allowed_marketplaces_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
        if let Some(plugin) = time_lock_plugin {
            plugin.run_asset_plugin(ctx)?;
        }

        Ok(true)
    }
    fn get_plugin_ctx<'a>(
        deps: Deps<'a>,
        env: &Env,
        info: &MessageInfo,
    ) -> PluginCtx<'a, DefaultXionAssetContext, Empty> {
        PluginCtx {
            deps,
            env: env.clone(),
            info: info.clone(),
            response: Response::default(),
            data: DefaultXionAssetContext::default(),
        }
    }
}

/// These are default plugins that can be used out of the box.
pub mod default_plugins {

    use super::*;
    use cosmwasm_std::{Attribute, Empty};

    /// opinionated plugin functions for some common actions
    /// e.g. listing an asset, delisting an asset, transferring an asset, buying an asset
    /// we only need to check things not already covered by the core logic

    /// this plugin checks that the price of purchase matches the ask price exactly
    /// if an ask price is set
    pub fn exact_price_plugin(
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
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
    pub fn min_price_plugin(
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
        // check if the minimum price is met
        if let Some(min_price) = &ctx.data.min_price {
            if let Some(ask_price) = ctx.data.ask_price.clone() {
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
    pub fn not_before_plugin(
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
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
    pub fn not_after_plugin(
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
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
    pub fn time_lock_plugin(
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
        if let Some(time_lock) = &ctx.data.time_lock {
            if let Some(reservation) = &ctx.data.reservation {
                if reservation.reserved_until
                    > Expiration::AtTime(ctx.env.block.time.plus_seconds(time_lock.as_secs()))
                {
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
        let (recipient, bps, on_primary) = match (
            ctx.data.nft_royalty_recipient.clone(),
            ctx.data.nft_royalty_bps,
            ctx.data.nft_royalty_on_primary,
        ) {
            (Some(recipient), Some(bps), on_primary) => (recipient, bps, on_primary),
            _ => match (
                ctx.data.collection_royalty_recipient.clone(),
                ctx.data.collection_royalty_bps,
                ctx.data.collection_royalty_on_primary,
            ) {
                (Some(recipient), Some(bps), on_primary) => (recipient, bps, on_primary),
                _ => return Ok(true),
            },
        };

        if bps == 0 {
            return Ok(true);
        }

        let is_primary_sale = !ctx.data.primary_complete;
        let collect_on_primary = on_primary.unwrap_or(false);
        let should_collect = !is_primary_sale || collect_on_primary;

        if !should_collect {
            return Ok(true);
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
            amount: vec![royalty_coin],
        }));

        ctx.response.messages.push(msg);

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

            if !allowed.iter().any(|addr| addr == &buyer) {
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
}

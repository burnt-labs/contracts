use std::{fmt::Display, time::Duration};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Binary, Coin, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
    StdResult, coin,
};
use cw721::{
    Expiration,
    error::Cw721ContractError,
    msg::Cw721ExecuteMsg,
    state::CREATOR,
    traits::{
        Cw721CustomMsg, Cw721Execute, Cw721State, FromAttributesState, StateFactory,
        ToAttributesState,
    },
};

use crate::{
    default_plugins::{self},
    error::ContractError,
    msg::{AssetExtensionExecuteMsg, ReserveMsg},
    state::AssetConfig,
    traits::{DefaultAssetContract, PluggableAsset, SellableAsset},
};

/// Shared context passed through the pipeline, mutated by plugins.
pub struct PluginCtx<'a, Context, TCustomResponseMsg>
where
    TCustomResponseMsg: CustomMsg,
{
    pub deps: Deps<'a>,
    pub env: Env,
    pub info: MessageInfo,

    // royalty info
    pub royalty: RoyaltyInfo,

    /// The response being built up by the plugins.
    pub response: Response<TCustomResponseMsg>,
    pub deductions: Vec<(String, Coin, String)>, // (recipient, amount, reason)

    pub data: Context,
}

#[derive(Default)]
pub struct RoyaltyInfo {
    pub collection_royalty_bps: Option<u16>,
    pub collection_royalty_recipient: Option<Addr>,

    pub primary_complete: bool,
}

pub struct DefaultXionAssetContext {
    pub token_id: String,
    pub seller: Option<Addr>,
    pub buyer: Option<Addr>,

    pub min_price: Option<Coin>, // minimum price an asset can be listed for
    pub ask_price: Option<Coin>, // if (List) or None on transfer

    pub not_before: Expiration, // timestamp before which an asset cannot be listed
    pub not_after: Expiration,  // timestamp after which an asset cannot be listed
    pub reservation: Option<ReserveMsg>,
    pub time_lock: Option<Duration>,

    pub allowed_marketplaces: Option<Vec<Addr>>,
    pub marketplace_fee_bps: Option<u16>,
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
            allowed_marketplaces: None,
            marketplace_fee_bps: None,
            allowed_currencies: None,
            time_lock: None,
        }
    }
}

pub type DefaultPluginCtx<'a> = PluginCtx<'a, DefaultXionAssetContext, Empty>;

#[cw_serde]
pub enum Plugin {
    ExactPrice { amount: Coin },
    MinimumPrice { amount: Coin },
    RequiresProof { proof: Vec<u8> },
    NotBefore { time: Expiration },
    NotAfter { time: Expiration },
    TimeLock { time: Duration },
    Royalty { bps: u16, recipient: Addr },
    AllowedMarketplaces { marketplaces: Vec<Addr> },
    AllowedCurrencies { denoms: Vec<Coin> },
}

impl Display for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Plugin::ExactPrice { amount } => write!(f, "ExactPrice: {}", amount),
            Plugin::MinimumPrice { amount } => write!(f, "MinimumPrice: {}", amount),
            Plugin::RequiresProof { proof } => write!(f, "RequiresProof: {:?}", proof),
            Plugin::NotBefore { time } => write!(f, "NotBefore: {}", time),
            Plugin::NotAfter { time } => write!(f, "NotAfter: {}", time),
            Plugin::TimeLock { time } => write!(f, "TimeLock: {:?}", time),
            Plugin::Royalty { bps, recipient } => {
                write!(f, "Royalty: {} bps to {}", bps, recipient)
            }
            Plugin::AllowedMarketplaces { marketplaces } => {
                write!(f, "AllowedMarketplaces: {:?}", marketplaces)
            }
            Plugin::AllowedCurrencies { denoms } => {
                write!(f, "AllowedCurrencies: {:?}", denoms)
            }
        }
    }
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
            Plugin::Royalty { bps, recipient } => {
                ctx.royalty.collection_royalty_bps = Some(*bps);
                ctx.royalty.collection_royalty_recipient = Some((*recipient).clone());
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

    pub fn run_raw_transfer_plugin<T, U: CustomMsg>(
        &self,
        ctx: &mut PluginCtx<T, U>,
    ) -> StdResult<bool> {
        if let Plugin::Royalty { bps, recipient } = self {
            ctx.royalty.collection_royalty_bps = Some(*bps);
            ctx.royalty.collection_royalty_recipient = Some((*recipient).clone());
            default_plugins::is_transfer_enabled_plugin(ctx)?;
        }
        Ok(true)
    }

    pub fn get_plugin_name(&self) -> &str {
        match self {
            Plugin::ExactPrice { .. } => "ExactPrice",
            Plugin::MinimumPrice { .. } => "MinimumPrice",
            Plugin::RequiresProof { .. } => "RequiresProof",
            Plugin::NotBefore { .. } => "NotBefore",
            Plugin::NotAfter { .. } => "NotAfter",
            Plugin::TimeLock { .. } => "TimeLock",
            Plugin::Royalty { .. } => "Royalty",
            Plugin::AllowedMarketplaces { .. } => "AllowedMarketplaces",
            Plugin::AllowedCurrencies { .. } => "AllowedCurrencies",
        }
    }
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
    fn execute_pluggable(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, AssetExtensionExecuteMsg>,
    ) -> Result<Response<Empty>, Cw721ContractError> {
        let plugin_response: Response<Empty>;
        let plugin_ctx_deductions: Vec<(String, Coin, String)>;
        {
            let mut plugin_ctx = Self::get_plugin_ctx(deps.as_ref(), env, info);

            match &msg {
                Cw721ExecuteMsg::TransferNft {
                    recipient,
                    token_id,
                } => self.on_transfer_plugin(recipient, token_id, &mut plugin_ctx)?,
                Cw721ExecuteMsg::SendNft {
                    contract, token_id, ..
                } => self.on_transfer_plugin(contract, token_id, &mut plugin_ctx)?,
                Cw721ExecuteMsg::UpdateExtension { msg } => {
                    self.on_update_extension_plugin(msg, &mut plugin_ctx)?
                }
                _ => true,
            };
            plugin_response = plugin_ctx.response;
            plugin_ctx_deductions = plugin_ctx.deductions.clone();
        };
        let mut response = match &msg {
            Cw721ExecuteMsg::UpdateExtension {
                msg:
                    AssetExtensionExecuteMsg::Buy {
                        token_id,
                        recipient,
                    },
            } => self.buy(
                deps,
                env,
                info,
                (*token_id).clone(),
                (*recipient).clone(),
                plugin_ctx_deductions,
            )?,
            _ => self.execute(deps, env, info, msg)?,
        };

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
                ..
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
            _ => Ok(true),
        }
    }

    fn on_list_plugin(
        &self,
        token_id: &str,
        price: &Coin,
        reservation: &Option<ReserveMsg>,
        ctx: &mut DefaultPluginCtx,
    ) -> StdResult<bool> {
        // for listings we run the minimum price, not before, not after plugins if set
        let config = AssetConfig::<TNftExtension>::default();
        let min_price_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::MinimumPrice {
                amount: coin(0, ""),
            }
            .get_plugin_name(),
        )?;
        let not_before_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::NotBefore {
                time: Expiration::Never {},
            }
            .get_plugin_name(),
        )?;
        let not_after_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::NotAfter {
                time: Expiration::Never {},
            }
            .get_plugin_name(),
        )?;
        let allowed_currencies_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::AllowedCurrencies { denoms: [].into() }.get_plugin_name(),
        )?;
        ctx.data.token_id = token_id.to_string();
        ctx.data.ask_price = Some(price.clone());
        ctx.data.marketplace_fee_bps = None;
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

    fn on_delist_plugin(&self, _token_id: &str, _ctx: &mut DefaultPluginCtx) -> StdResult<bool> {
        Ok(true)
    }

    fn on_buy_plugin(
        &self,
        token_id: &str,
        _recipient: &Option<String>,
        ctx: &mut DefaultPluginCtx,
    ) -> StdResult<bool> {
        // for buys we run the exact price, then allowed marketplaces and royalty plugins if set
        let config = AssetConfig::<TNftExtension>::default();
        let allowed_marketplaces_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::AllowedMarketplaces {
                marketplaces: [].into(),
            }
            .get_plugin_name(),
        )?;
        let allowed_currencies_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::AllowedCurrencies { denoms: [].into() }.get_plugin_name(),
        )?;
        let royalty_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "Royalty")?;
        ctx.data.token_id = token_id.to_string();
        ctx.data.buyer = Some(ctx.info.sender.clone());
        ctx.data.marketplace_fee_bps = None;
        // we need to get the listing info to get the ask price
        let listing = self
            .config
            .listings
            .load(ctx.deps.storage, token_id)
            .map_err(|_| ContractError::ListingNotFound {
                id: token_id.to_string(),
            })?;
        ctx.data.ask_price = Some(listing.price.clone());
        if let Some(plugin) = allowed_currencies_plugin {
            plugin.run_asset_plugin(ctx)?;
        }
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
        token_id: &str,
        reservation: &ReserveMsg,
        ctx: &mut PluginCtx<DefaultXionAssetContext, Empty>,
    ) -> StdResult<bool> {
        let config = AssetConfig::<TNftExtension>::default();
        // we run the allowed marketplaces and not_after plugin if set
        ctx.data.buyer = ctx.info.sender.clone().into();
        ctx.data.reservation = Some(reservation.clone());
        ctx.data.token_id = token_id.to_string();
        let allowed_marketplaces_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::AllowedMarketplaces {
                marketplaces: [].into(),
            }
            .get_plugin_name(),
        )?;
        let time_lock_plugin = config.collection_plugins.may_load(
            ctx.deps.storage,
            Plugin::TimeLock {
                time: Duration::from_secs(0),
            }
            .get_plugin_name(),
        )?;
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
            royalty: RoyaltyInfo::default(),
            data: DefaultXionAssetContext::default(),
            deductions: vec![],
        }
    }

    fn save_plugin(
        &self,
        deps: DepsMut,
        _env: &Env,
        info: &MessageInfo,
        plugins: &[Plugin],
    ) -> StdResult<()> {
        CREATOR
            .assert_owner(deps.storage, &info.sender)
            .map_err(|err| StdError::generic_err(err.to_string()))?;
        for plugin in plugins {
            self.config
                .collection_plugins
                .save(deps.storage, plugin.get_plugin_name(), plugin)?;
        }
        Ok(())
    }

    fn remove_plugin(
        &self,
        deps: DepsMut,
        _env: &Env,
        info: &MessageInfo,
        plugins: &[String],
    ) -> StdResult<()> {
        CREATOR
            .assert_owner(deps.storage, &info.sender)
            .map_err(|err| StdError::generic_err(err.to_string()))?;
        for plugin in plugins {
            self.config.collection_plugins.remove(deps.storage, plugin);
        }
        Ok(())
    }
}

use std::marker::PhantomData;

use crate::{
    error::ContractError,
    execute::{buy, delist, list, reserve, unreserve},
    msg::{AssetExtensionExecuteMsg, AssetExtensionQueryMsg, ReserveMsg},
    plugin::{Plugin, PluginCtx},
    state::AssetConfig,
};
use cosmwasm_std::{
    Binary, Coin, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult,
    to_json_binary,
};
use cw_storage_plus::Bound;
use cw721::{
    error::Cw721ContractError,
    msg::Cw721ExecuteMsg,
    traits::{
        Contains, Cw721CustomMsg, Cw721Execute, Cw721Query, Cw721State, FromAttributesState,
        StateFactory, ToAttributesState,
    },
};

pub struct AssetContract<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TExtensionMsg,
> where
    TNftExtension: Cw721State,
{
    pub config: AssetConfig<'a, TNftExtension>,
    pub(crate) _collection_extension: PhantomData<TCollectionExtension>,
    pub(crate) _nft_extension_msg: PhantomData<TNftExtensionMsg>,
    pub(crate) _collection_extension_msg: PhantomData<TCollectionExtensionMsg>,
    pub(crate) _extension_msg: PhantomData<TExtensionMsg>,
    pub(crate) _extension_query_msg: PhantomData<Empty>,
    pub(crate) _custom_response_msg: PhantomData<Empty>,
}
impl<TNftExtension, TNftExtensionMsg, TCollectionExtension, TCollectionExtensionMsg, TExtentionMsg>
    Default
    for AssetContract<
        'static,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtentionMsg,
    >
where
    TNftExtension: Cw721State,
    TNftExtensionMsg: Cw721CustomMsg + StateFactory<TNftExtension>,
    TCollectionExtension: Cw721State + ToAttributesState + FromAttributesState,
    TCollectionExtensionMsg: Cw721CustomMsg + StateFactory<TCollectionExtension>,
{
    fn default() -> Self {
        AssetContract {
            config: Default::default(),
            _collection_extension: PhantomData,
            _nft_extension_msg: PhantomData,
            _collection_extension_msg: PhantomData,
            _extension_msg: PhantomData,
            _extension_query_msg: PhantomData,
            _custom_response_msg: PhantomData,
        }
    }
}

pub type DefaultAssetContract<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
> = AssetContract<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    AssetExtensionExecuteMsg,
>;

impl<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TExtentionMsg,
>
    AssetContract<
        'a,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtentionMsg,
    >
where
    TNftExtension: Cw721State,
    TCollectionExtension: Cw721State,
    TCollectionExtensionMsg: Default,
{
    pub fn new(config: AssetConfig<'a, TNftExtension>) -> Self {
        AssetContract {
            config,
            _collection_extension: PhantomData,
            _nft_extension_msg: PhantomData,
            _collection_extension_msg: PhantomData,
            _extension_msg: PhantomData,
            _extension_query_msg: PhantomData,
            _custom_response_msg: PhantomData,
        }
    }
}

pub trait SellableAsset<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TExtentionMsg,
    TCustomResponseMsg,
> where
    TNftExtension: Cw721State,
    TNftExtensionMsg: StateFactory<TNftExtension> + Cw721CustomMsg,
    TCollectionExtension: Cw721State,
    TCollectionExtension: FromAttributesState + ToAttributesState,
    TCollectionExtensionMsg: StateFactory<TCollectionExtension> + Cw721CustomMsg,
    TCollectionExtensionMsg: Default,
    TCustomResponseMsg: CustomMsg,
{
    fn list(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        id: String,
        price: Coin,
        reservation: Option<ReserveMsg>,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        list::<TNftExtension, TCustomResponseMsg>(deps, env, info, id, price, reservation)
    }
    fn delist(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        id: String,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        delist::<TNftExtension, TCustomResponseMsg>(deps, env, info, id)
    }
    fn reserve(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        id: String,
        reservation: ReserveMsg,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        reserve::<TNftExtension, TCustomResponseMsg>(deps, env, info, id, reservation)
    }
    fn unreserve(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        id: String,
        delist: bool,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        unreserve::<TNftExtension, TCustomResponseMsg>(deps, env, info, id, delist)
    }

    fn buy(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        id: String,
        recipient: Option<String>,
        deductions: Vec<(String, Coin, String)>,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        buy::<TNftExtension, TCustomResponseMsg>(deps, env, info, id, recipient, deductions)
    }
}

impl<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TExtentionMsg,
>
    SellableAsset<
        'a,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtentionMsg,
        Empty,
    >
    for AssetContract<
        'a,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtentionMsg,
    >
where
    TNftExtension: Cw721State,
    TNftExtensionMsg: StateFactory<TNftExtension> + Cw721CustomMsg,
    TCollectionExtension: Cw721State,
    TCollectionExtension: FromAttributesState + ToAttributesState,
    TCollectionExtensionMsg: StateFactory<TCollectionExtension> + Cw721CustomMsg,
    TCollectionExtensionMsg: Default,
{
}

impl<TNftExtension, TNftExtensionMsg, TCollectionExtension, TCollectionExtensionMsg>
    Cw721Execute<
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
    TNftExtension: Cw721State,
    TNftExtensionMsg: StateFactory<TNftExtension> + Cw721CustomMsg,
    TCollectionExtensionMsg: StateFactory<TCollectionExtension> + Cw721CustomMsg,
    TCollectionExtension: Cw721State,
    TCollectionExtension: FromAttributesState + ToAttributesState,
    TCollectionExtensionMsg: Default,
{
    fn execute_extension(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: AssetExtensionExecuteMsg,
    ) -> Result<Response<Empty>, cw721::error::Cw721ContractError> {
        match msg {
            AssetExtensionExecuteMsg::List {
                token_id,
                price,
                reservation,
            } => Ok(self.list(deps, env, info, token_id, price, reservation)?),
            AssetExtensionExecuteMsg::Reserve {
                token_id,
                reservation,
            } => Ok(self.reserve(deps, env, info, token_id, reservation)?),
            AssetExtensionExecuteMsg::UnReserve { token_id, delist } => {
                Ok(self.unreserve(deps, env, info, token_id, delist.unwrap_or(false))?)
            }
            AssetExtensionExecuteMsg::Delist { token_id } => {
                Ok(self.delist(deps, env, info, token_id)?)
            }
            AssetExtensionExecuteMsg::Buy {
                token_id,
                recipient,
            } => Ok(self.buy(deps, env, info, token_id, recipient, [].into())?),
            AssetExtensionExecuteMsg::SetCollectionPlugin { plugins } => {
                self.save_plugin(deps, env, info, &plugins)?;
                Ok(Response::new().add_attribute(
                    "action",
                    format!("set_collection_plugin {:?}", plugins.clone()),
                ))
            }
            AssetExtensionExecuteMsg::RemoveCollectionPlugin { plugins } => {
                self.remove_plugin(deps, env, info, &plugins)?;
                Ok(Response::new()
                    .add_attribute("action", format!("remove_collection_plugin {:?}", plugins)))
            }
        }
    }
}

impl<TNftExtension, TNftExtensionMsg, TCollectionExtension, TCollectionExtensionMsg>
    Cw721Query<TNftExtension, TCollectionExtension, AssetExtensionQueryMsg>
    for DefaultAssetContract<
        '_,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
    >
where
    TNftExtension: Cw721State + Contains,
    TCollectionExtension: Cw721State + FromAttributesState + ToAttributesState,
{
    fn query_extension(
        &self,
        deps: cosmwasm_std::Deps,
        _env: &Env,
        msg: AssetExtensionQueryMsg,
    ) -> Result<cosmwasm_std::Binary, cw721::error::Cw721ContractError> {
        match msg {
            AssetExtensionQueryMsg::GetListing { token_id } => {
                let listing = self
                    .config
                    .listings
                    .may_load(deps.storage, &token_id)?
                    .ok_or(ContractError::ListingNotFound {
                        id: token_id.clone(),
                    })?;
                Ok(to_json_binary(&listing)?)
            }
            AssetExtensionQueryMsg::GetListingsBySeller {
                seller,
                start_after,
                limit,
            } => {
                let seller_addr = deps.api.addr_validate(&seller)?;
                let listings: Vec<_> = self
                    .config
                    .listings
                    .idx
                    .seller
                    .prefix(seller_addr)
                    .range(
                        deps.storage,
                        start_after.map(|s| Bound::ExclusiveRaw(s.into())),
                        None,
                        cosmwasm_std::Order::Ascending,
                    )
                    .take(limit.unwrap_or(10) as usize)
                    .map(|item| item.map(|(_, listing)| listing))
                    .collect::<Result<_, _>>()?;
                Ok(to_json_binary(&listings)?)
            }
            AssetExtensionQueryMsg::GetCollectionPlugins {} => {
                let plugins: Vec<_> = self
                    .config
                    .collection_plugins
                    .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
                    .map(|item| item.map(|(_, plugin)| plugin))
                    .collect::<Result<_, _>>()?;
                Ok(to_json_binary(&plugins)?)
            }
            AssetExtensionQueryMsg::GetAllListings { start_after, limit } => {
                let listings: Vec<_> = self
                    .config
                    .listings
                    .range(
                        deps.storage,
                        start_after.map(|s| Bound::ExclusiveRaw(s.into())),
                        None,
                        cosmwasm_std::Order::Ascending,
                    )
                    .take(limit.unwrap_or(10) as usize)
                    .map(|item| item.map(|(_, listing)| listing))
                    .collect::<Result<_, _>>()?;
                Ok(to_json_binary(&listings)?)
            }
        }
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
                Cw721ExecuteMsg::SendNft {
                    contract, token_id, ..
                } => self.on_transfer_plugin(contract, token_id, &mut plugin_ctx)?,
                Cw721ExecuteMsg::UpdateExtension { msg } => {
                    self.on_update_extension_plugin(msg, &mut plugin_ctx)?
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
        _recipient: &str,
        _token_id: &str,
        ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        // for transfers we run the royalty plugin if set
        let config = AssetConfig::<TNftExtension>::default();
        let royalty_plugin = config
            .collection_plugins
            .may_load(ctx.deps.storage, "Royalty")?;
        if let Some(plugin) = royalty_plugin {
            plugin.run_raw_transfer_plugin(ctx)?;
        }
        if AssetConfig::<TNftExtension>::default()
            .listings
            .may_load(ctx.deps.storage, _token_id)?
            .is_some()
        {
            return Err(StdError::generic_err(
                "cannot transfer a token while it is listed",
            ));
        }
        Ok(true)
    }

    fn on_update_extension_plugin(
        &self,
        _msg: &TExtensionMsg,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_list_plugin(
        &self,
        _token_id: &str,
        _price: &Coin,
        _reservation: &Option<ReserveMsg>,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_delist_plugin(
        &self,
        _token_id: &str,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_buy_plugin(
        &self,
        _token_id: &str,
        _recipient: &Option<String>,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn on_reserve_plugin(
        &self,
        _token_id: &str,
        _reserver: &ReserveMsg,
        _ctx: &mut PluginCtx<Context, TCustomResponseMsg>,
    ) -> StdResult<bool> {
        Ok(true)
    }

    fn get_plugin_ctx<'a>(
        deps: Deps<'a>,
        env: &Env,
        info: &MessageInfo,
    ) -> PluginCtx<'a, Context, TCustomResponseMsg>;

    fn save_plugin(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        plugins: &[Plugin],
    ) -> StdResult<()>;

    fn remove_plugin(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        plugins: &[String],
    ) -> StdResult<()>;
}

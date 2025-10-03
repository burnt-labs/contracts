use std::marker::PhantomData;

use crate::{
    error::ContractError,
    execute::{buy, delist, list, reserve},
    msg::{AssetExtensionExecuteMsg, AssetExtensionQueryMsg},
    plugin::PluggableAsset,
    state::{AssetConfig, Reserve},
};
use cosmwasm_std::{to_json_binary, Addr, Coin, CustomMsg, DepsMut, Empty, Env, MessageInfo, Response};
use cw_storage_plus::Bound;
use cw721::traits::{
    Contains, Cw721CustomMsg, Cw721Execute, Cw721Query, Cw721State, FromAttributesState,
    StateFactory, ToAttributesState,
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
        reservation: Option<Reserve>,
        marketplace_fee_bps: Option<u16>,
        marketplace_fee_recipient: Option<String>,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        list::<TNftExtension, TCustomResponseMsg>(
            deps,
            env,
            info,
            id,
            price,
            reservation,
            marketplace_fee_bps,
            marketplace_fee_recipient,
        )
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
        reservation: Reserve,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        reserve::<TNftExtension, TCustomResponseMsg>(deps, env, info, id, reservation)
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

impl<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
>
    Cw721Execute<
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        AssetExtensionExecuteMsg,
        Empty,
    >
    for DefaultAssetContract<
        'a,
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
                marketplace_fee_bps,
                marketplace_fee_recipient,
            } => Ok(self.list(deps, env, info, token_id, price, reservation, marketplace_fee_bps, marketplace_fee_recipient)?),
            AssetExtensionExecuteMsg::Reserve {
                token_id,
                reservation,
            } => Ok(self.reserve(deps, env, info, token_id, reservation)?),
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

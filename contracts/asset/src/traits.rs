use std::marker::PhantomData;

use crate::{
    error::ContractError,
    execute::{buy, delist, list, reserve},
    msg::AssetExtensionExecuteMsg,
    state::{AssetConfig, Reserve},
};
use cosmwasm_std::{Coin, CustomMsg, DepsMut, Empty, Env, MessageInfo, Response};
use cw721::{
    traits::{
        Cw721CustomMsg, Cw721Execute, Cw721State, FromAttributesState, StateFactory,
        ToAttributesState,
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
        reservation: Option<Reserve>,
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
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        buy::<TNftExtension, TCustomResponseMsg>(deps, env, info, id, recipient)
    }
}

impl<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TExtentionMsg,
    TCustomResponseMsg,
>
    SellableAsset<
        'a,
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        TExtentionMsg,
        TCustomResponseMsg,
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
    TCustomResponseMsg: CustomMsg,
{
}

impl<
    'a,
    TNftExtension,
    TNftExtensionMsg,
    TCollectionExtension,
    TCollectionExtensionMsg,
    TCustomResponseMsg,
>
    Cw721Execute<
        TNftExtension,
        TNftExtensionMsg,
        TCollectionExtension,
        TCollectionExtensionMsg,
        AssetExtensionExecuteMsg,
        TCustomResponseMsg,
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
    TCustomResponseMsg: CustomMsg,
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
    ) -> Result<Response<TCustomResponseMsg>, cw721::error::Cw721ContractError> {
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
            AssetExtensionExecuteMsg::Delist { token_id } => {
                Ok(self.delist(deps, env, info, token_id)?)
            }
            AssetExtensionExecuteMsg::Buy {
                token_id,
                recipient,
            } => Ok(self.buy(deps, env, info, token_id, recipient)?),
        }
    }
}

use std::marker::PhantomData;

use crate::{
    execute::{buy, delist, list, reserve},
    msg::{AssetExtensionExecuteMsg, XionAssetCollectionMetadataMsg},
    state::{AssetConfig, XionAssetCollectionMetadata},
};
use cosmwasm_std::{CustomMsg, DepsMut, Empty, Env, MessageInfo, Response};
use cw721::{
    DefaultOptionalNftExtension,
    traits::{
        Cw721CustomMsg, Cw721Execute, Cw721State, FromAttributesState, StateFactory,
        ToAttributesState,
    },
};

pub struct AssetContract<'a, TNftExtension, TCollectionExtension, TCollectionExtensionMsg>
where
    TNftExtension: Cw721State,
{
    pub config: AssetConfig<'a, TNftExtension>,
    pub(crate) _collection_extension: PhantomData<TCollectionExtension>,
    pub(crate) _nft_extension_msg: PhantomData<TNftExtension>,
    pub(crate) _collection_extension_msg: PhantomData<TCollectionExtensionMsg>,
    pub(crate) _extension_msg: PhantomData<AssetExtensionExecuteMsg>,
    pub(crate) _extension_query_msg: PhantomData<Empty>,
    pub(crate) _custom_response_msg: PhantomData<Empty>,
}
impl Default
    for AssetContract<
        'static,
        DefaultOptionalNftExtension,
        XionAssetCollectionMetadata,
        XionAssetCollectionMetadataMsg,
    >
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

impl<'a, TNftExtension, TCollectionExtension, TCollectionExtensionMsg>
    AssetContract<'a, TNftExtension, TCollectionExtension, TCollectionExtensionMsg>
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
    > for AssetContract<'a, TNftExtension, TCollectionExtension, TCollectionExtensionMsg>
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
                id,
                price,
                reservation,
            } => Ok(list::<TNftExtension, TCustomResponseMsg>(
                deps,
                env,
                info,
                id,
                price,
                reservation,
            )?),
            AssetExtensionExecuteMsg::Reserve { id, reservation } => {
                Ok(reserve::<TNftExtension, TCustomResponseMsg>(
                    deps,
                    env,
                    info,
                    id,
                    reservation,
                )?)
            }
            AssetExtensionExecuteMsg::Delist { id } => Ok(delist::<
                TNftExtension,
                TCustomResponseMsg,
            >(deps, env, info, id)?),
            AssetExtensionExecuteMsg::Buy { id, recipient } => {
                Ok(buy::<TNftExtension, TCustomResponseMsg>(
                    deps, env, info, id, recipient,
                )?)
            }
        }
    }
}

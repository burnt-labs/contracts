use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Empty};
use cw721::state::Cw721Config;
use cw721::{
    DefaultOptionalCollectionExtension, DefaultOptionalCollectionExtensionMsg,
    DefaultOptionalNftExtension, DefaultOptionalNftExtensionMsg,
};
use std::marker::PhantomData;

pub struct Cw721Proxy<'a> {
    pub config: Cw721Config<'a, DefaultOptionalNftExtension>,
    pub(crate) _collection_extension: PhantomData<DefaultOptionalCollectionExtension>,
    pub(crate) _nft_extension_msg: PhantomData<DefaultOptionalNftExtensionMsg>,
    pub(crate) _collection_extension_msg: PhantomData<DefaultOptionalCollectionExtensionMsg>,
    pub(crate) _extension_msg: PhantomData<Empty>,
    pub(crate) _extension_query_msg: PhantomData<Empty>,
    pub(crate) _custom_response_msg: PhantomData<Empty>,
}

pub struct ProxyMsg {
    sender: Addr,
    msg: Binary,
}

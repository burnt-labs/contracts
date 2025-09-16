use std::fmt::Debug;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw721::traits::{FromAttributesState, ToAttributesState};
use serde::{de::DeserializeOwned, Serialize};

pub trait Extensions: Serialize + DeserializeOwned + Clone + Debug + ToAttributesState + FromAttributesState {}

/// Default collection extension metadata
#[cw_serde]
pub struct DefaultXionAssetCollectionMetadataMsg {
    // Royalty basis points (out of 10,000)
    pub royalty_bps: Option<u16>,
    // Royalty recipient address
    pub royalty_recipient: Option<Addr>,
    // Whether royalties are taken on primary sales
    pub royalty_on_primary: Option<bool>,
    // Minimum listing price for assets in this collection
    pub min_list_price: Option<u128>,
    // Optional time lock before which assets cannot be listed
    pub not_before: Option<u64>,
    // Optional time lock after which assets cannot be listed
    pub not_after: Option<u64>,
    // List of plugins to enable for this collection
    pub plugins: Vec<String>,
}

pub type InstantiateMsg<CollectionExtension: Extensions> = cw721::msg::Cw721InstantiateMsg<CollectionExtension>;

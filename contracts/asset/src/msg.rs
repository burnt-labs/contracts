use std::fmt::Debug;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr};
use cw721::{error::Cw721ContractError, traits::{Cw721CustomMsg, FromAttributesState, StateFactory, ToAttributesState}};
use serde::{de::DeserializeOwned, Serialize};

use crate::state::XionAssetCollectionMetadata;

pub trait Extensions: Serialize + DeserializeOwned + Clone + Debug + ToAttributesState + FromAttributesState {}
/// Default collection extension metadata
#[cw_serde]
pub struct XionAssetCollectionMetadataMsg {
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

impl FromAttributesState for XionAssetCollectionMetadataMsg{
    fn from_attributes_state(_: &[cw721::Attribute]) -> Result<Self, Cw721ContractError> { todo!() }
}

impl ToAttributesState for XionAssetCollectionMetadataMsg {
    fn to_attributes_state(&self) -> Result<std::vec::Vec<cw721::Attribute>, Cw721ContractError> { todo!() }
}

impl StateFactory<XionAssetCollectionMetadata> for XionAssetCollectionMetadataMsg {
    fn create(
        &self,
        deps: cosmwasm_std::Deps,
        env: &cosmwasm_std::Env,
        info: Option<&cosmwasm_std::MessageInfo>,
        current: Option<&XionAssetCollectionMetadata>,
    ) -> Result<XionAssetCollectionMetadata, Cw721ContractError> {
        Ok(XionAssetCollectionMetadata {
            royalty_bps: self.royalty_bps,
            royalty_recipient: self.royalty_recipient.clone(),
            royalty_on_primary: self.royalty_on_primary,
            min_list_price: self.min_list_price,
            not_before: self.not_before,
            not_after: self.not_after,
            plugins: self.plugins.clone(),
        })
    }

    fn validate(
        &self,
        deps: cosmwasm_std::Deps,
        env: &cosmwasm_std::Env,
        info: Option<&cosmwasm_std::MessageInfo>,
        current: Option<&XionAssetCollectionMetadata>,
    ) -> Result<(), Cw721ContractError> {
        Ok(())
    }
}

impl Cw721CustomMsg for XionAssetCollectionMetadataMsg {}

pub type InstantiateMsg<CollectionExtension: Extensions> = cw721::msg::Cw721InstantiateMsg<CollectionExtension>;

#[cw_serde]
pub enum XionAssetExtensionExecuteMsg{
    List { id: String, price: u128 },
    FreezeListing { id: String },
    Delist { id: String },
    Buy { id: String, recipient: Option<String> },
}

pub type ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtentionMsg> = cw721::msg::Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtentionMsg>;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, from_json, to_json_binary};
use cw_storage_plus::{IndexList, IndexedMap, Item, Map, MultiIndex};
use cw721::{
    Expiration,
    state::{Cw721Config, NftInfo},
    traits::{Cw721State, FromAttributesState, ToAttributesState},
};

use crate::plugin::Plugin;

#[cw_serde]
pub struct XionAssetCollectionMetadata {
    pub royalty_bps: Option<u16>,
    pub royalty_recipient: Option<Addr>,
    pub royalty_on_primary: Option<bool>,
    pub min_list_price: Option<u128>,
    pub not_before: Option<u64>,
    pub not_after: Option<u64>,
    pub plugins: Vec<String>,
}

impl FromAttributesState for XionAssetCollectionMetadata {
    fn from_attributes_state(
        attrs: &[cw721::Attribute],
    ) -> Result<Self, cw721::error::Cw721ContractError> {
        let mut royalty_bps: Option<u16> = None;
        let mut royalty_recipient: Option<Addr> = None;
        let mut royalty_on_primary: Option<bool> = None;
        let mut min_list_price: Option<u128> = None;
        let mut not_before: Option<u64> = None;
        let mut not_after: Option<u64> = None;
        let mut plugins: Vec<String> = vec![];

        for attr in attrs {
            match attr.key.as_str() {
                "royalty_bps" => {
                    royalty_bps = Some(from_json(attr.value.clone())?);
                }
                "royalty_recipient" => {
                    royalty_recipient =
                        Some(Addr::unchecked(from_json::<String>(attr.value.clone())?));
                }
                "royalty_on_primary" => {
                    royalty_on_primary = Some(from_json(attr.value.clone())?);
                }
                "min_list_price" => {
                    min_list_price = Some(from_json(attr.value.clone())?);
                }
                "not_before" => {
                    not_before = Some(from_json(attr.value.clone())?);
                }
                "not_after" => {
                    not_after = Some(from_json(attr.value.clone())?);
                }
                "plugins" => {
                    plugins = from_json(attr.value.clone())?;
                }
                _ => {}
            }
        }

        Ok(XionAssetCollectionMetadata {
            royalty_bps,
            royalty_recipient,
            royalty_on_primary,
            min_list_price,
            not_before,
            not_after,
            plugins,
        })
    }
}

impl ToAttributesState for XionAssetCollectionMetadata {
    fn to_attributes_state(
        &self,
    ) -> Result<Vec<cw721::Attribute>, cw721::error::Cw721ContractError> {
        let mut attrs: Vec<cw721::Attribute> = vec![];

        if let Some(bps) = self.royalty_bps {
            attrs.push(cw721::Attribute {
                key: "royalty_bps".to_string(),
                value: to_json_binary(&bps)?,
            });
        }

        if let Some(recipient) = &self.royalty_recipient {
            attrs.push(cw721::Attribute {
                key: "royalty_recipient".to_string(),
                value: to_json_binary(recipient)?,
            });
        }

        if let Some(on_primary) = self.royalty_on_primary {
            attrs.push(cw721::Attribute {
                key: "royalty_on_primary".to_string(),
                value: to_json_binary(&on_primary)?,
            });
        }

        if let Some(price) = self.min_list_price {
            attrs.push(cw721::Attribute {
                key: "min_list_price".to_string(),
                value: to_json_binary(&price)?,
            });
        }

        if let Some(nb) = self.not_before {
            attrs.push(cw721::Attribute {
                key: "not_before".to_string(),
                value: to_json_binary(&nb)?,
            });
        }

        if let Some(na) = self.not_after {
            attrs.push(cw721::Attribute {
                key: "not_after".to_string(),
                value: to_json_binary(&na)?,
            });
        }

        if !self.plugins.is_empty() {
            attrs.push(cw721::Attribute {
                key: "plugins".to_string(),
                value: to_json_binary(&self.plugins)?,
            });
        }

        Ok(attrs)
    }
}

impl Cw721State for XionAssetCollectionMetadata {}

#[cw_serde]
pub struct ListingInfo<TNftExtension> {
    pub id: String,
    pub price: Coin,
    pub seller: Addr,
    pub reserved: Option<Reserve>,
    pub nft_info: NftInfo<TNftExtension>,
}

#[cw_serde]
pub struct Reserve {
    pub reserver: Addr,
    pub reserved_until: Expiration,
}

pub struct AssetConfig<'a, TNftExtension>
where
    TNftExtension: Cw721State,
{
    pub listings:
        IndexedMap<&'a str, ListingInfo<TNftExtension>, ListingIndexes<'a, TNftExtension>>,
    pub collection_plugins: Map<&'a str, Plugin>,
    /// We create a reference to the cw721 states
    pub cw721_config: Cw721Config<'a, TNftExtension>,
}

impl<TNftExtension> Default for AssetConfig<'static, TNftExtension>
where
    TNftExtension: Cw721State,
{
    fn default() -> Self {
        Self::new(
            "listings_token_info",
            "listings_token_info__by_seller",
            "collection_plugins",
        )
    }
}

impl<'a, TNftExtension> AssetConfig<'a, TNftExtension>
where
    TNftExtension: Cw721State,
{
    pub fn new(
        listing_info_key: &'static str,
        listing_info_seller_key: &'static str,
        collection_plugins: &'static str,
    ) -> Self {
        let listing_indexes = ListingIndexes {
            seller: MultiIndex::new(seller_index, listing_info_key, listing_info_seller_key),
        };
        Self {
            listings: IndexedMap::new(listing_info_key, listing_indexes),
            collection_plugins: Map::new(collection_plugins),
            cw721_config: Cw721Config::default(),
        }
    }
}

pub fn seller_index<TNftExtension>(_pk: &[u8], d: &ListingInfo<TNftExtension>) -> Addr {
    d.seller.clone()
}

pub struct ListingIndexes<'a, TNftExtension> {
    pub seller: MultiIndex<'a, Addr, ListingInfo<TNftExtension>, String>,
}

impl<'a, TNftExtension> IndexList<ListingInfo<TNftExtension>> for ListingIndexes<'a, TNftExtension>
where
    TNftExtension: Cw721State,
{
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<ListingInfo<TNftExtension>>> + '_>
    {
        let v: Vec<&dyn cw_storage_plus::Index<ListingInfo<TNftExtension>>> = vec![&self.seller];
        Box::new(v.into_iter())
    }
}

// Collection-wide plugins
pub const COLLECTION_PLUGINS_ID: Item<Vec<String>> = Item::new("collection_plugins");

// Optional token-specific overrides
pub const TOKEN_PLUGINS: Map<&str, Vec<String>> = Map::new("token_plugins");

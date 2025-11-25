use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::{IndexList, IndexedMap, Map, MultiIndex};
use cw721::{state::Cw721Config, traits::Cw721State};

use crate::plugin::Plugin;

#[cw_serde]
pub struct ListingInfo {
    pub id: String,
    pub price: Coin,
    pub seller: Addr,
    pub reserved: Option<Reserve>,
}

#[cw_serde]
pub struct Reserve {
    pub reserver: Addr,
    pub reserved_until: Timestamp,
}

pub struct AssetConfig<'a, TNftExtension>
where
    TNftExtension: Cw721State,
{
    pub listings: IndexedMap<&'a str, ListingInfo, ListingIndexes<'a>>,
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

impl<TNftExtension> AssetConfig<'_, TNftExtension>
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

pub fn seller_index(_pk: &[u8], d: &ListingInfo) -> Addr {
    d.seller.clone()
}

pub struct ListingIndexes<'a> {
    pub seller: MultiIndex<'a, Addr, ListingInfo, String>,
}

impl IndexList<ListingInfo> for ListingIndexes<'_> {
    fn get_indexes(
        &'_ self,
    ) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<ListingInfo>> + '_> {
        let v: Vec<&dyn cw_storage_plus::Index<ListingInfo>> = vec![&self.seller];
        Box::new(v.into_iter())
    }
}

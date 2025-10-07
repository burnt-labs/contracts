use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{IndexList, IndexedMap, Item, Map, MultiIndex};
use cw721::{
    Expiration,
    state::{Cw721Config, NftInfo},
    traits::{Cw721State},
};

use crate::plugin::Plugin;

#[cw_serde]
pub struct ListingInfo<TNftExtension> {
    pub id: String,
    pub price: Coin,
    pub seller: Addr,
    pub reserved: Option<Reserve>,
    pub nft_info: NftInfo<TNftExtension>,
    pub marketplace_fee_bps: Option<u16>,
    pub marketplace_fee_recipient: Option<Addr>,
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

use cosmwasm_schema::cw_serde;

use crate::error::ContractError;
use cosmwasm_std::{ensure, Addr, Api, Coin, Storage};
use cw_address_like::AddressLike;
use cw_storage_plus::{index_list, IndexedMap, Item, MultiIndex};

#[cw_serde]
pub struct Config<T: AddressLike> {
    pub manager: T,
    pub sale_approvals: bool,
    pub fee_bps: u64,
    pub listing_denom: String,
}

// Maximum fee bps allowed.
const MAX_FEE_BPS: u64 = 10_000;

impl Config<Addr> {
    pub fn save(&self, storage: &mut dyn Storage) -> Result<(), ContractError> {
        ensure!(
            self.fee_bps <= MAX_FEE_BPS,
            ContractError::InvalidFeeRate {}
        );
        CONFIG.save(storage, self)?;
        Ok(())
    }
}

impl Config<String> {
    pub fn to_addr(&self, api: &dyn Api) -> Result<Config<Addr>, ContractError> {
        Ok(Config {
            manager: api.addr_validate(&self.manager)?,
            fee_bps: self.fee_bps,
            sale_approvals: self.sale_approvals,
            listing_denom: self.listing_denom.clone(),
        })
    }
}
impl Config<Addr> {
    pub fn from_str(config: Config<String>, api: &dyn Api) -> Result<Self, ContractError> {
        Ok(Config {
            manager: api.addr_validate(&config.manager)?,
            fee_bps: config.fee_bps,
            sale_approvals: config.sale_approvals,
            listing_denom: config.listing_denom,
        })
    }
}
impl From<Config<Addr>> for Config<String> {
    fn from(config: Config<Addr>) -> Self {
        Config {
            manager: config.manager.to_string(),
            fee_bps: config.fee_bps,
            sale_approvals: config.sale_approvals,
            listing_denom: config.listing_denom,
        }
    }
}

pub const CONFIG: Item<Config<Addr>> = Item::new("config");

#[cw_serde]
pub enum ListingStatus {
    Active,
    PendingApproval,
}

impl std::fmt::Display for ListingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListingStatus::Active => write!(f, "Active"),
            ListingStatus::PendingApproval => write!(f, "PendingApproval"),
        }
    }
}

#[cw_serde]
pub struct Listing {
    pub id: String,
    pub collection: Addr,
    pub token_id: String,
    pub price: Coin,
    pub seller: Addr,
    pub status: ListingStatus,
}

type ListingId = String;
type OfferId = String;
type CollectionOfferId = String;
#[index_list(Listing)]
pub struct ListingIndices<'a> {
    pub by_seller: MultiIndex<'a, Addr, Listing, ListingId>,
}

pub fn listings<'a>() -> IndexedMap<ListingId, Listing, ListingIndices<'a>> {
    let listing_indices = ListingIndices {
        by_seller: MultiIndex::new(
            |_pk: &[u8], listing: &Listing| listing.seller.clone(),
            "l",
            "ls",
        ),
    };
    IndexedMap::new("l", listing_indices)
}

#[cw_serde]
pub struct Offer {
    pub id: String,
    pub buyer: Addr,
    pub price: Coin,
    pub collection: Addr,
    pub token_id: String,
}

#[index_list(Offer)]
pub struct OfferIndices<'a> {
    pub by_collection_and_price: MultiIndex<'a, (Addr, String, u128), Offer, OfferId>,
}

pub fn offers<'a>() -> IndexedMap<OfferId, Offer, OfferIndices<'a>> {
    let offer_indices = OfferIndices {
        by_collection_and_price: MultiIndex::new(
            |_pk: &[u8], offer: &Offer| {
                (
                    offer.collection.clone(),
                    offer.price.denom.clone(),
                    offer.price.amount.u128(),
                )
            },
            "o",   // offers namespace shorter for storage efficiency
            "ocp", // offers by collection and price
        ),
    };
    IndexedMap::new("o", offer_indices)
}

#[cw_serde]
pub struct CollectionOffer {
    pub id: String,
    pub buyer: Addr,
    pub price: Coin,
    pub collection: Addr,
}

#[index_list(CollectionOffer)]
pub struct CollectionOfferIndices<'a> {
    pub by_collection_and_price:
        MultiIndex<'a, (Addr, String, u128), CollectionOffer, CollectionOfferId>,
}

pub fn collection_offers<'a>(
) -> IndexedMap<CollectionOfferId, CollectionOffer, CollectionOfferIndices<'a>> {
    let collection_offer_indices = CollectionOfferIndices {
        by_collection_and_price: MultiIndex::new(
            |_pk: &[u8], collection_offer: &CollectionOffer| {
                (
                    collection_offer.collection.clone(),
                    collection_offer.price.denom.clone(),
                    collection_offer.price.amount.u128(),
                )
            },
            "co",  // collection offers namespace shorter for storage efficiency
            "cop", // collection offers by collection and price
        ),
    };
    IndexedMap::new("co", collection_offer_indices)
}

pub const AUTO_INCREMENT: Item<u64> = Item::new("auto_increment");

// next_auto_increment is inteded to be used as a generator nonce for unique ids in combination
// with other sources of entropy to generate unique ids.
pub fn next_auto_increment(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let auto_increment = AUTO_INCREMENT.load(storage)?.wrapping_add(1);
    AUTO_INCREMENT.save(storage, &auto_increment)?;
    Ok(auto_increment)
}

pub fn init_auto_increment(storage: &mut dyn Storage) -> Result<(), ContractError> {
    AUTO_INCREMENT.save(storage, &0)?;
    Ok(())
}

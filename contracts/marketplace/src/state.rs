use cosmwasm_schema::cw_serde;

use crate::error::ContractError;
use cosmwasm_std::{ensure, Addr, Api, Coin, Storage};
use cw_address_like::AddressLike;
use cw_storage_plus::{index_list, IndexedMap, Item, MultiIndex, UniqueIndex};

#[cw_serde]
pub struct Config<T: AddressLike> {
    pub manager: T,
    pub fee_recipient: T,
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
    pub fn validate(&self) -> Result<(), ContractError> {
        ensure!(
            self.fee_bps <= MAX_FEE_BPS,
            ContractError::InvalidFeeRate {}
        );

        ensure!(
            !self.listing_denom.is_empty(),
            ContractError::InvalidListingDenom {
                expected: "non-empty".to_string(),
                actual: self.listing_denom.clone(),
            }
        );
        ensure!(
            !self.fee_recipient.is_empty(),
            ContractError::InvalidFeeRecipient {}
        );
        Ok(())
    }
    pub fn to_addr(&self, api: &dyn Api) -> Result<Config<Addr>, ContractError> {
        Ok(Config {
            manager: api.addr_validate(&self.manager)?,
            fee_recipient: api.addr_validate(&self.fee_recipient)?,
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
            fee_recipient: api.addr_validate(&config.fee_recipient)?,
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
            fee_recipient: config.fee_recipient.to_string(),
            sale_approvals: config.sale_approvals,
            listing_denom: config.listing_denom,
        }
    }
}

pub const CONFIG: Item<Config<Addr>> = Item::new("config");

#[cw_serde]
pub enum ListingStatus {
    Active,
    Reserved,
}

impl std::fmt::Display for ListingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListingStatus::Active => write!(f, "Active"),
            ListingStatus::Reserved => write!(f, "Reserved"),
        }
    }
}

#[cw_serde]
pub struct Listing {
    pub id: String,
    pub collection: Addr,
    pub token_id: String,
    pub price: Coin,
    pub asset_price: Coin,
    pub seller: Addr,
    pub reserved_for: Option<Addr>,
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
// with other sources of entropy to generate unique ids, and never to be used as source of unique ids
// as it wraps around effectily resetting the counter.
pub fn next_auto_increment(storage: &mut dyn Storage) -> Result<u64, ContractError> {
    let auto_increment = AUTO_INCREMENT.load(storage)?.wrapping_add(1);
    AUTO_INCREMENT.save(storage, &auto_increment)?;
    Ok(auto_increment)
}

pub fn init_auto_increment(storage: &mut dyn Storage) -> Result<(), ContractError> {
    AUTO_INCREMENT.save(storage, &0)?;
    Ok(())
}

#[cw_serde]
pub enum SaleType {
    BuyNow,
    TokenOffer,
    CollectionOffer,
}

type PendingSaleId = String;
#[cw_serde]
pub struct PendingSale {
    pub id: String,
    pub collection: Addr,
    pub token_id: String,
    pub price: Coin,
    pub seller: Addr,
    pub buyer: Addr,
    pub recipient: Addr,
    pub sale_type: SaleType,
    pub time: u64,
    pub expiration: u64,
}

#[index_list(PendingSale)]
pub struct PendingSaleIndices<'a> {
    pub by_seller: MultiIndex<'a, Addr, PendingSale, PendingSaleId>,
    pub by_buyer: MultiIndex<'a, Addr, PendingSale, PendingSaleId>,
    pub by_collection_and_token_id: UniqueIndex<'a, (Addr, String), PendingSale, PendingSaleId>,
    pub by_expiration: MultiIndex<'a, u64, PendingSale, PendingSaleId>,
}

const PENDING_SALES_NAMESPACE: &str = "ps";
pub fn pending_sales<'a>() -> IndexedMap<PendingSaleId, PendingSale, PendingSaleIndices<'a>> {
    let pending_sale_indices = PendingSaleIndices {
        by_seller: MultiIndex::new(
            |_id, pending_sale: &PendingSale| pending_sale.seller.clone(),
            PENDING_SALES_NAMESPACE,
            "pss", // pending sale seller index namespace
        ),
        by_buyer: MultiIndex::new(
            |_id, pending_sale: &PendingSale| pending_sale.buyer.clone(),
            PENDING_SALES_NAMESPACE,
            "psb", // pending sale buyer index namespace
        ),
        by_collection_and_token_id: UniqueIndex::new(
            |pending_sale: &PendingSale| {
                (
                    pending_sale.collection.clone(),
                    pending_sale.token_id.clone(),
                )
            },
            "psct", // pending sale collection and token id index namespace
        ),
        by_expiration: MultiIndex::new(
            |_id, pending_sale: &PendingSale| pending_sale.expiration,
            PENDING_SALES_NAMESPACE,
            "pse", // pending sale expiration index namespace
        ),
    };
    IndexedMap::new("ps", pending_sale_indices)
}

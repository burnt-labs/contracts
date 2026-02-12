use crate::state::{CollectionOffer, Config, Listing, Offer, PendingSale};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    ListItem {
        price: Coin,
        collection: String,
        token_id: String,
        reserved_for: Option<String>,
    },
    CancelListing {
        listing_id: String,
    },
    BuyItem {
        listing_id: String,
        price: Coin,
    },
    FinalizeFor {
        listing_id: String,
        price: Coin,
        recipient: String,
    },
    CreateOffer {
        collection: String,
        token_id: String,
        price: Coin,
    },
    AcceptOffer {
        id: String,
        collection: String,
        token_id: String,
        price: Coin,
    },
    CreateCollectionOffer {
        collection: String,
        price: Coin,
    },
    AcceptCollectionOffer {
        id: String,
        collection: String,
        token_id: String,
        price: Coin,
    },
    CancelOffer {
        id: String,
    },
    CancelCollectionOffer {
        id: String,
    },
    ApproveSale {
        id: String,
    },
    RejectSale {
        id: String,
    },
    ReclaimExpiredSale {
        id: String,
    },
    UpdateConfig {
        config: Config<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config<Addr>)]
    Config {},
    #[returns(Listing)]
    Listing { listing_id: String },
    #[returns(Offer)]
    Offer { offer_id: String },
    #[returns(CollectionOffer)]
    CollectionOffer { collection_offer_id: String },
    #[returns(PendingSale)]
    PendingSale { id: String },
    #[returns(Vec<PendingSale>)]
    PendingSales {
        limit: Option<u32>,
        start_after: Option<u64>,
    },
    #[returns(Vec<PendingSale>)]
    PendingSalesByExpiry {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

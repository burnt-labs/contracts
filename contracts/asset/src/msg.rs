use crate::plugin::Plugin;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Timestamp};
use cw721::traits::Cw721CustomMsg;

pub type InstantiateMsg<CollectionExtension> = cw721::msg::Cw721InstantiateMsg<CollectionExtension>;

#[cw_serde]
pub struct ReserveMsg {
    pub reserver: Option<String>,
    pub reserved_until: Timestamp,
}
#[cw_serde]
pub enum AssetExtensionExecuteMsg {
    List {
        token_id: String,
        price: Coin,
        reservation: Option<ReserveMsg>,
    },
    Reserve {
        token_id: String,
        reservation: ReserveMsg,
    },
    UnReserve {
        token_id: String,
        delist: Option<bool>,
    },
    Delist {
        token_id: String,
    },
    Buy {
        token_id: String,
        recipient: Option<String>,
    },
    SetCollectionPlugin {
        plugins: Vec<Plugin>,
    },
    RemoveCollectionPlugin {
        plugins: Vec<String>,
    },
}

#[cw_serde]
pub enum AssetExtensionQueryMsg {
    GetListing {
        token_id: String,
    },
    GetListingsBySeller {
        seller: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetAllListings {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetCollectionPlugins {},
}
impl Cw721CustomMsg for AssetExtensionQueryMsg {}

pub type ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg> =
    cw721::msg::Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg>;

pub type QueryMsg<TNftExtension, TCollectionExtension, TExtensionQueryMsg> =
    cw721::msg::Cw721QueryMsg<TNftExtension, TCollectionExtension, TExtensionQueryMsg>;

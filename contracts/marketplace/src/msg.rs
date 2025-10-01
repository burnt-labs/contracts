use crate::state::Config;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    pub config: Config<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateListing {
        price: Coin,
        collection: String,
        token_id: String,
    },
    CancelListing {
        collection: String,
        token_id: String,
    },
    PurchaseItem {
        collection: String,
        token_id: String,
        price: Coin,
    },
    CreateOffer {
        collection: String,
        token_id: String,
        price: Coin,
    },
    AcceptOffer {
        id: String,
    },
    CreateCollectionOffer {
        collection: String,
        price: Coin,
    },
    AcceptCollectionOffer {
        id: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config<Addr>)]
    Config {},
}

#[cw_serde]
pub struct MigrateMsg {}

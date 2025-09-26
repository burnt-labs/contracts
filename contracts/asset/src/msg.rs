use crate::state::Reserve;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

pub type InstantiateMsg<CollectionExtension> = cw721::msg::Cw721InstantiateMsg<CollectionExtension>;

#[cw_serde]
pub enum AssetExtensionExecuteMsg {
    List {
        token_id: String,
        price: Coin,
        reservation: Option<Reserve>,
    },
    Reserve {
        token_id: String,
        reservation: Reserve,
    },
    Delist {
        token_id: String,
    },
    Buy {
        token_id: String,
        recipient: Option<String>,
    },
}

pub type ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg> =
    cw721::msg::Cw721ExecuteMsg<TNftExtensionMsg, TCollectionExtensionMsg, TExtensionMsg>;

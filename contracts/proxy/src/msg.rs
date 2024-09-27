use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin, CosmosMsg, WasmMsg};


#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<Addr>,
    pub code_ids: Vec<u64>,
}

#[cw_serde]
pub enum ExecuteMsg {
    ProxyMsgs {
        msgs: Vec<WasmMsg>,
    },
    UpdateAdmin {
        new_admin: Option<Addr>,
    },
    AddCodeIDs {
        code_ids: Vec<u64>,
    },
    RemoveCodeIDs {
        code_ids: Vec<u64>,
    }
}

#[cw_serde]
pub struct ProxyMsg {
    pub sender: Addr,
    pub msg: Binary,
    pub funds: Vec<Coin>,
}
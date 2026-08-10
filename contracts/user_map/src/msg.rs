use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    Update { value: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Addr>)]
    GetUsers {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(String)]
    GetValueByUser { address: Addr },
    #[returns(Vec<(Addr, String)>)]
    GetMap {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

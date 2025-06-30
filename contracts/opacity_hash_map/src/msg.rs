use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use opacity_verifier::msg::QueryMsg::Verify;

#[cw_serde]
pub struct InstantiateMsg {
    pub opacity_verifier: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    Update { message: String, signature: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Addr>)]
    GetUsers {},
    #[returns(String)]
    GetValueByUser { address: Addr },
    #[returns(Vec<(Addr, String)>)]
    GetMap {},
}

#[cw_serde]
pub struct MigrateMsg {}

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use reclaim_xion::msg::ProofMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub verification_addr: Addr,
    pub claim_key: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Update { value: ProofMsg },
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
    #[returns(String)]
    GetClaimKey {},
}

#[cw_serde]
pub struct MigrateMsg {}

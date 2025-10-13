use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use ios_app_attest::msg::VerifyAttestation;

#[cw_serde]
pub struct InstantiateMsg {
    pub app_id: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Update { attestation: VerifyAttestation },
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

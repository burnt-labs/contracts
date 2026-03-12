use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use ios_app_attest::msg::VerifyAttestation;

use crate::state::BacResponse;

#[cw_serde]
pub struct InstantiateMsg {
    pub app_id: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Update { attestation: VerifyAttestation },
    Sweep {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Addr>)]
    GetUsers {},
    #[returns(BacResponse)]
    GetValueByUser { address: Addr },
    #[returns(Vec<(Addr, BacResponse)>)]
    GetMap {},
}

#[cw_serde]
pub struct MigrateMsg {}

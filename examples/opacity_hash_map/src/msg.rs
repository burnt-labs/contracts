use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use serde_json::Value;

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
    // Value, not String: the handlers store and return whatever JSON the
    // attested message parsed to, so an object or an array is as likely as a
    // string. Declaring String here told generated clients to expect a quoted
    // string and made them fail to deserialize anything else.
    #[returns(Value)]
    GetValueByUser { address: Addr },
    #[returns(Vec<(Addr, Value)>)]
    GetMap {},
}

#[cw_serde]
pub struct MigrateMsg {}

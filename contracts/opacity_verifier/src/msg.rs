use cosmwasm_std::Addr;
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Addr,
    pub allow_list: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateAdmin { admin: Addr },
    UpdateAllowList { keys: Vec<String> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(bool)]
    Verify {
        signature: String,
        message: String,
    },
    
    #[returns(Vec<String>)]
    VerificationKeys {},
    
    #[returns(Addr)]
    Admin {}
}
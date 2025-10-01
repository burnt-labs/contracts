use crate::ark_verifier::{SnarkJsProof, SnarkJsVkey};
use cosmwasm_schema::{QueryResponses, cw_serde};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    pub vkey: SnarkJsVkey,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Binary)]
    Verify {
        proof: Box<SnarkJsProof>,
        dkim_domain: String,
        tx_bytes: Binary,
        email_hash: Binary,
        dkim_hash: Binary,
    },

    #[returns(Binary)]
    VKey {},
}

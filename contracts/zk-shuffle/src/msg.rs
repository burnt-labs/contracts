//! Minimal zkShuffle contract for testing proof verification at XION module level

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint256;

use crate::types::Groth16Proof;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    VerifyShuffleProof {
        proof: Groth16Proof,
        public_inputs: Vec<Uint256>,
    },
    VerifyDecryptProof {
        proof: Groth16Proof,
        public_inputs: Vec<Uint256>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(VerificationCountResponse)]
    VerificationCount {},
}

#[cw_serde]
pub struct VerificationCountResponse {
    pub shuffle_verifications: u64,
    pub decrypt_verifications: u64,
}

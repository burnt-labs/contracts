use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint256;

#[cw_serde]
pub struct Groth16Proof {
    pub a: [Uint256; 2],
    pub b: [[Uint256; 2]; 2],
    pub c: [Uint256; 2],
}

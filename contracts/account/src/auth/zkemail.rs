use crate::error::ContractResult;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, Addr, Binary, Deps};

#[cw_serde]
pub struct ZKEmailSignature {
    proof: zkemail::ark_verifier::SnarkJsProof,
    dkim_hash: Binary,
}

pub fn verify(
    deps: Deps,
    verification_contract: &Addr,
    tx_bytes: &Binary,
    sig_bytes: &Binary,
    email_hash: &Binary,
    dkim_domain: &str,
) -> ContractResult<bool> {
    let sig: ZKEmailSignature = from_json(sig_bytes)?;

    let verification_request = zkemail::msg::QueryMsg::Verify {
        proof: Box::new(sig.proof),
        dkim_domain: dkim_domain.to_owned(),
        tx_bytes: tx_bytes.clone(),
        email_hash: email_hash.clone(),
        dkim_hash: sig.dkim_hash,
    };

    let verified: bool = deps
        .querier
        .query_wasm_smart(verification_contract, &verification_request)?;

    Ok(verified)
}

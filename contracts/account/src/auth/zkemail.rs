use crate::error::ContractResult;
use cosmos_sdk_proto::{
    traits::MessageExt,
    xion::v1::dkim::{QueryVerifyRequest, QueryVerifyResponse},
};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, Addr, Binary, Deps};

#[cw_serde]
pub struct SnarkJsProof {
    pi_a: [String; 3],
    pi_b: [[String; 2]; 3],
    pi_c: [String; 3],
}
#[cw_serde]
pub struct ZKEmailSignature {
    proof: SnarkJsProof,
    dkim_hash: Binary,
}

pub fn verify(
    deps: Deps,
    tx_bytes: &Binary,
    sig_bytes: &Binary,
    email_hash: &Binary,
    dkim_domain: &str,
) -> ContractResult<bool> {
    let sig: ZKEmailSignature = from_json(sig_bytes.clone())?;

    let verification_request = QueryVerifyRequest {
        proof: sig_bytes.to_vec(),
        dkim_domain: dkim_domain.to_owned(),
        tx_bytes: tx_bytes.to_vec(),
        email_hash: email_hash.to_vec(),
        dkim_hash: sig.dkim_hash.to_vec(),
    };
    let verification_request_byte = verification_request.to_bytes()?;
    let verification_response: Binary = deps.querier.query_grpc(
        "/xion.dkim.v1.Query/ProofVerify".to_string(),
        Binary::from(verification_request_byte),
    )?;

    let res: QueryVerifyResponse = from_json(verification_response)?;

    Ok(res.verified)
}

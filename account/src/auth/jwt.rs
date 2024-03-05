use crate::error::ContractResult;
use crate::proto::{self, XionCustomQuery};
use base64::engine::general_purpose;
use base64::Engine as _;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Deps;
use std::str;

#[cw_serde]
struct QueryValidateJWTResponse {}

pub fn verify(
    deps: Deps<XionCustomQuery>,
    tx_hash: &Vec<u8>,
    sig_bytes: &[u8],
    aud: &str,
    sub: &str,
) -> ContractResult<bool> {
    let challenge = general_purpose::STANDARD.encode(tx_hash);

    let query = proto::QueryValidateJWTRequest {
        aud: aud.to_string(),
        sub: sub.to_string(),
        sigBytes: String::from_utf8(sig_bytes.into()).unwrap(),
        txHash: challenge,
    };

    deps.querier
        .query::<QueryValidateJWTResponse>(&query.into())?;

    Ok(true)
}

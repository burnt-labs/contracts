use crate::error::ContractResult;
use crate::proto::{self, XionCustomQuery};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Deps};

#[cw_serde]
struct QueryRegisterRequest {
    addr: String,
    challenge: String,
    rp: String,
    data: Binary,
}

#[cw_serde]
struct QueryRegisterResponse {
    credential: Binary,
}

pub fn register(
    deps: Deps<XionCustomQuery>,
    addr: Addr,
    rp: String,
    data: Binary,
) -> ContractResult<Binary> {
    let query = proto::QueryWebAuthNVerifyRegisterRequest {
        addr: addr.clone().into(),
        challenge: Binary::from(addr.as_bytes()).to_base64(),
        rp,
        data: data.to_vec(),
    };

    let query_response = deps.querier.query::<QueryRegisterResponse>(&query.into())?;

    Ok(query_response.credential)
}

#[cw_serde]
struct QueryVerifyRequest {
    addr: String,
    challenge: String,
    rp: String,
    credential: Binary,
    data: Binary,
}

pub fn verify(
    deps: Deps<XionCustomQuery>,
    addr: Addr,
    rp: String,
    signature: &Binary,
    tx_hash: Vec<u8>,
    credential: &Binary,
) -> ContractResult<bool> {
    let challenge = Binary::from(tx_hash).to_base64();

    let query = proto::QueryWebAuthNVerifyAuthenticateRequest {
        addr: addr.into(),
        challenge,
        rp,
        credential: credential.clone().into(),
        data: signature.clone().into(),
    };

    deps.querier.query::<QueryRegisterResponse>(&query.into())?;

    Ok(true)
}

use crate::error::ContractResult;
use crate::proto::{self, QueryWebAuthNVerifyRegisterRequest, QueryWebAuthNVerifyRegisterResponse};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::QueryRequest::{Custom, Stargate};
use cosmwasm_std::{to_binary, Addr, Binary, Deps};

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

pub fn register(deps: Deps, addr: Addr, rp: String, data: Binary) -> ContractResult<Binary> {
    let query = QueryRegisterRequest {
        addr: addr.clone().into(),
        challenge: addr.to_string(),
        rp,
        data,
    };
    let query_bz = to_binary(&query)?;

    let query_response: QueryRegisterResponse = deps.querier.query(&Stargate {
        path: "xion.v1.Query/WebAuthNVerifyRegister".to_string(),
        data: query_bz,
    })?;

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
    deps: Deps,
    addr: Addr,
    rp: String,
    signature: &Binary,
    tx_hash: Vec<u8>,
    credential: &Binary,
) -> ContractResult<bool> {
    let challenge = URL_SAFE_NO_PAD.encode(tx_hash);

    let query = QueryVerifyRequest {
        addr: addr.into(),
        challenge,
        rp,
        credential: credential.clone(),
        data: signature.clone(),
    };
    let query_bz = to_binary(&query)?;

    deps.querier.query(&Stargate {
        path: "xion.v1.Query/WebAuthNVerifyAuthenticate".to_string(),
        data: query_bz,
    })?;

    Ok(true)
}

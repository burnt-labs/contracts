use crate::error::ContractError;
use crate::error::ContractResult;
use crate::msg::{InstantiateMsg, QueryMsg, VerifyResponse};
use crate::state::AUD;
use crate::msg::IntegrityVerdict;
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response,
};

/// Manual prost definitions for VerifyJWS types not yet in the published
/// xion-cosmos-sdk-proto crate.
mod proto {
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct QueryVerifyJwsRequest {
        #[prost(string, tag = "1")]
        pub aud: String,
        #[prost(string, tag = "2")]
        pub sig_bytes: String,
    }

    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct QueryVerifyJwsResponse {
        #[prost(bytes = "vec", tag = "1")]
        pub payload: Vec<u8>,
    }
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    AUD.save(deps.storage, &msg.aud)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("aud", msg.aud))
}

#[entry_point]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> ContractResult<Response> {
    Err(ContractError::Std(cosmwasm_std::StdError::generic_err(
        "no execute messages",
    )))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Verify { compact_jws } => query_verify(deps, compact_jws),
        QueryMsg::GetAud {} => {
            let aud = AUD.load(deps.storage)?;
            Ok(to_json_binary(&aud)?)
        }
    }
}

fn query_verify(deps: Deps, compact_jws: String) -> ContractResult<Binary> {
    let aud = AUD.load(deps.storage)?;

    // Delegate signature verification to xion's JWK module.
    // It looks up the audience's public key and verifies the JWS,
    // returning the verified payload bytes.
    let req = proto::QueryVerifyJwsRequest {
        aud,
        sig_bytes: compact_jws,
    };
    let req_bz = prost::Message::encode_to_vec(&req);
    let resp_bz = deps.querier.query_grpc(
        String::from("/xion.jwk.v1.Query/VerifyJWS"),
        Binary::new(req_bz),
    )?;

    let resp: proto::QueryVerifyJwsResponse = prost::Message::decode(resp_bz.as_slice())
        .map_err(|e| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "failed to decode VerifyJWS response: {}",
                e
            )))
        })?;

    // Parse the verified payload into the verdict
    let verdict: IntegrityVerdict = serde_json::from_slice(&resp.payload)?;

    Ok(to_json_binary(&VerifyResponse { verdict })?)
}

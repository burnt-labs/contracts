use crate::ark_verifier::{SnarkJsProof, SnarkJsVkey};
use crate::commit::calculate_tx_body_commitment;
use crate::error::ContractError::InvalidDkim;
use crate::error::ContractResult;
use crate::groth16::{GrothBn, GrothFp};
use crate::msg::QueryMsg::VKey;
use crate::msg::{InstantiateMsg, QueryMsg};
use crate::state::VKEY;
use crate::{CONTRACT_NAME, CONTRACT_VERSION};
use ark_crypto_primitives::snark::SNARK;
use ark_ff::Zero;
use ark_serialize::CanonicalDeserialize;
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::xion::v1::dkim::{QueryDkimPubKeysRequest, QueryDkimPubKeysResponse};
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, Storage, entry_point, to_json_binary,
};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    init(deps, env, msg.vkey)
}

pub fn init(deps: DepsMut, env: Env, vkey: SnarkJsVkey) -> ContractResult<Response> {
    VKEY.save(deps.storage, &vkey)?;

    Ok(
        Response::new().add_event(Event::new("create_abstract_account").add_attributes(vec![
            ("contract_address", env.contract.address.to_string()),
            ("vkey", serde_json::to_string(&vkey)?),
        ])),
    )
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        VKey {} => query_vkey(deps.storage),
        QueryMsg::Verify {
            proof,
            tx_bytes,
            email_hash,
            dkim_domain,
            dkim_hash,
        } => query_verify(
            deps,
            *proof,
            &tx_bytes,
            &email_hash,
            &dkim_domain,
            &dkim_hash,
        ),
    }
}

fn query_vkey(store: &dyn Storage) -> ContractResult<Binary> {
    let vkey = VKEY.load(store)?;
    Ok(to_json_binary(&vkey)?)
}

fn query_verify(
    deps: Deps,
    proof: SnarkJsProof,
    tx_bytes: &Binary,
    email_hash: &Binary,
    dkim_domain: &String,
    dkim_hash: &Binary,
) -> ContractResult<Binary> {
    let vkey = VKEY.load(deps.storage)?;

    // verify that domain+hash are known in chain state
    let query = QueryDkimPubKeysRequest {
        selector: "".to_string(), // do not filter on selector
        domain: dkim_domain.to_string(),
        poseidon_hash: dkim_hash.to_vec(),
        pagination: None,
    };
    let query_bz = query.to_bytes()?;
    let query_response = deps.querier.query_grpc(
        String::from("/xion.dkim.v1.Query/QueryDkimPubKeys"),
        Binary::new(query_bz),
    )?;
    let query_response = QueryDkimPubKeysResponse::decode(query_response.as_slice())?;
    if query_response.dkim_pub_keys.is_empty() {
        return Err(InvalidDkim);
    }

    // inputs are tx body, email hash, and dmarc key hash
    let mut inputs: [GrothFp; 3] = [GrothFp::zero(); 3];

    // tx body input
    let tx_input = calculate_tx_body_commitment(STANDARD_NO_PAD.encode(tx_bytes).as_str());
    inputs[0] = tx_input;

    // email hash input, compressed at authenticator registration
    let email_hash_input = GrothFp::deserialize_compressed(email_hash.as_slice())?;
    inputs[1] = email_hash_input;

    // verify the dkim pubkey hash in the proof output. the poseidon hash is
    // from the tx, we can't be sure if it was properly formatted
    inputs[2] = GrothFp::deserialize_compressed(dkim_hash.as_slice())?;

    let verified = GrothBn::verify(&vkey.into(), inputs.as_slice(), &proof.into())?;

    Ok(to_json_binary(&verified)?)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{testing::MockApi, Uint256};

    use super::*;

    #[test]
    fn verifying_zkemail_signature() {
        let api = MockApi::default();
        
        // build tx bytes to sign
        
        // load proof from previously sent and proved email

        // assign email salt from email used to prove
        
        // mock api for querying dkim module
        
        // submit data for verification

    }
}

use std::backtrace;

use crate::ark_verifier::{SnarkJsProof, SnarkJsVkey};
use crate::commit::calculate_tx_body_commitment;
use crate::error::ContractError::{InvalidDkim, Std};
use crate::error::ContractResult;
use crate::groth16::{GrothBn, GrothFp};
use crate::msg::QueryMsg::VKey;
use crate::msg::{InstantiateMsg, QueryMsg};
use crate::state::VKEY;
use crate::{CONTRACT_NAME, CONTRACT_VERSION};
use ark_crypto_primitives::snark::SNARK;
use ark_ff::Zero;
use ark_serialize::CanonicalDeserialize;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::xion::v1::dkim::{QueryDkimPubKeysRequest, QueryDkimPubKeysResponse};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, Storage,
};

#[cfg_attr(not(feature = "library"), entry_point)]
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

#[cfg_attr(not(feature = "library"), entry_point)]
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
        String::from("/xion.dkim.v1.Query/DkimPubKeys"),
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
    use std::{marker::PhantomData, str::FromStr};

    use cosmos_sdk_proto::xion::v1::dkim::DkimPubKey;
    use cosmwasm_std::{
        from_json,
        testing::{mock_env, MockApi, MockQuerier, MockStorage},
        ContractResult, Empty, OwnedDeps, Querier, QuerierResult, QuerierWrapper, SystemResult,
    };

    use super::*;

    struct CustomQuery {}

    impl Querier for CustomQuery {
        fn raw_query(&self, _bin_request: &[u8]) -> QuerierResult {
            // return google.com dkim response
            let res: QueryDkimPubKeysResponse = QueryDkimPubKeysResponse {
                dkim_pub_keys: vec![
                    DkimPubKey { domain: "google.com".to_string(), pub_key: "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4zd3nfUoLHWFbfoPZzAb8bvjsFIIFsNypweLuPe4M".to_string(), poseidon_hash: Binary::from_base64("Im03SDCYWJPZWS2pqo0Q2iHy6iZxKmkumas/jelebro=").unwrap().into(), selector: "20230601".to_string(), version: 0, key_type: 0 }
                ],
                pagination: None,
            };
            SystemResult::Ok(ContractResult::Ok(res.to_bytes().unwrap().into()))
        }
    }

    #[test]
    fn verifying_zkemail_signature() {
        let vkey_64 = Binary::from_base64("eyJ2a19hbHBoYV8xIjpbIjIwNDkxMTkyODA1MzkwNDg1Mjk5MTUzMDA5NzczNTk0NTM0OTQwMTg5MjYxODY2MjI4NDQ3OTE4MDY4NjU4NDcxOTcwNDgxNzYzMDQyIiwiOTM4MzQ4NTM2MzA1MzI5MDIwMDkxODM0NzE1NjE1NzgzNjU2NjU2Mjk2Nzk5NDAzOTcxMjI3MzQ0OTkwMjYyMTI2NjE3ODU0NTk1OCIsIjEiXSwidmtfYmV0YV8yIjpbWyI2Mzc1NjE0MzUxNjg4NzI1MjA2NDAzOTQ4MjYyODY4OTYyNzkzNjI1NzQ0MDQzNzk0MzA1NzE1MjIyMDExNTI4NDU5NjU2NzM4NzMxIiwiNDI1MjgyMjg3ODc1ODMwMDg1OTEyMzg5Nzk4MTQ1MDU5MTM1MzUzMzA3MzQxMzE5Nzc3MTc2ODY1MTQ0MjY2NTc1MjI1OTM5NzEzMiJdLFsiMTA1MDUyNDI2MjYzNzAyNjIyNzc1NTI5MDEwODIwOTQzNTY2OTc0MDk4MzU2ODAyMjA1OTA5NzE4NzMxNzExNDAzNzEzMzEyMDY4NTYiLCIyMTg0NzAzNTEwNTUyODc0NTQwMzI4ODIzMjY5MTE0NzU4NDcyODE5MTE2MjczMjI5OTg2NTMzODM3NzE1OTY5MjM1MDA1OTEzNjY3OSJdLFsiMSIsIjAiXV0sInZrX2dhbW1hXzIiOltbIjEwODU3MDQ2OTk5MDIzMDU3MTM1OTQ0NTcwNzYyMjMyODI5NDgxMzcwNzU2MzU5NTc4NTE4MDg2OTkwNTE5OTkzMjg1NjU1ODUyNzgxIiwiMTE1NTk3MzIwMzI5ODYzODcxMDc5OTEwMDQwMjEzOTIyODU3ODM5MjU4MTI4NjE4MjExOTI1MzA5MTc0MDMxNTE0NTIzOTE4MDU2MzQiXSxbIjg0OTU2NTM5MjMxMjM0MzE0MTc2MDQ5NzMyNDc0ODkyNzI0Mzg0MTgxOTA1ODcyNjM2MDAxNDg3NzAyODA2NDkzMDY5NTgxMDE5MzAiLCI0MDgyMzY3ODc1ODYzNDMzNjgxMzMyMjAzNDAzMTQ1NDM1NTY4MzE2ODUxMzI3NTkzNDAxMjA4MTA1NzQxMDc2MjE0MTIwMDkzNTMxIl0sWyIxIiwiMCJdXSwidmtfZGVsdGFfMiI6W1siMjIyMDk5OTM2NTkyMTI0NDExNTEwNDM1MDY3NzExNDc4NjM0NDc3Mjc1MjM4OTI4NzQyNDYzODU5NjAwODg0OTQ1NzQ1NDc2MTI4OCIsIjgxNTU0MDM1OTc1NTIwODk4NzExODA2MTg2MzUwNjI0OTgwMTQxOTY0OTU4MDUyMDk5NzYwOTIzODQ2NTMxOTcyOTk2MDkwNTg4MzkiXSxbIjg1ODg4ODM3MTU2OTQ3Njk2NTA2ODcxNzkzNzM1OTkzNzY3ODE3NDkyNjUwMjgzMTA2NDcyODU2NzYyMzAwMTA4MzA1NDAyNjE1NTQiLCI2ODY4NTgwNTE0NTY0MzYwNTE5NDQyMjAxMTU4NzYxNDAyNzY5NDcwMzI3MDc2OTQwMDYzNTcxMDk4MTQyNTI4NDc0MDQ2NjU5MTkzIl0sWyIxIiwiMCJdXSwiSUMiOltbIjEzNTUyNzk2MTU5MzIxNTAwNDQ2NTQyMDkyMzMzOTI1Nzg5NjI3MjIxNzYzOTk1OTE3MDU3NzA1MDI1MDI0NzM2ODQ1NzIxOTQyMzUzIiwiOTY4MDk4NTI4NTA0MDIzMDc1MTY0NDQ5NTg5MzIxNTM4MjY3NjA4MTUzNTg3NjM2MzU1MzY2MDM3NDExNTU4ODg2MjA2MDQ4NzAzNCIsIjEiXSxbIjE1MjU0Nzc2NzcyNjEwNTMzMzI3NDg4ODE0MjgxOTMxMjg3NjY1ODkzOTQzMTc3OTc3MzEwMTY4MDU2NzIyMzY4MjgxNzI4NjE4NjYyIiwiMTQzNjc2NTc3MDEyNDk1NDc5MTg1MTM2NzU1MzMwOTEyODYzODMzNTgxODU2NjI4Njk4ODczMTY0NjY4MTMwNjIxNDI2NzQ0OTkyNjAiLCIxIl0sWyIxNDg2Njg1OTE3Nzc1ODYzNTAzMDA3OTIyNjM0MTg2MjYwMTExMjk4MzQ4NzgzODUxMTY5MDU2Nzg5NTU3NDAzMDY3MjIxMjQ0MTY3NiIsIjExMzE0NTQyMjkzNTMzOTczMzI4NDE2NDQxNjYzMTc2MDMyMzk5ODQ1NTE5MzAwNDQ2ODk0NDA3NTIwMDgzNDEyMDM2ODg4MzQyOTgiLCIxIl0sWyIxNDQ1MjkwMjQ4MjgyODU1ODI3MDI5ODM3NDczNzcwNTY1ODM1MTI3MDc4ODQyODAxOTU4NTI0MTAyNjkwMTMzMTM2ODg1ODUyNzk0OCIsIjE5MDk3OTk3Nzc0NjA2NTIzNTI3MTU4NTI5NTUyNjQ1OTYxMjU3MTUwMTg5MzU3MzU3NjAxMDM1MDc4MTExNjQzOTUzMzg1MTcxMzg0IiwiMSJdXX0=").unwrap();
        let vkey: SnarkJsVkey = from_json(vkey_64).unwrap();
        // build tx bytes to sign
        let tx_bytes = Binary::from_base64("CqIBCp8BChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEn8KP3hpb24xdHRrMjR4MnhoazNrcDhxNnd1ZHZ6ZHN4eGo4dWU0NWxjZjdjZDVrcGhueGd4bmw1NDg5cW1sbmt0cBIreGlvbjFxYWYyeGZseDVqM2FndGx2cWs1dmhqcGV1aGw2ZzQ1aHhzaHdxahoPCgV1eGlvbhIGMTAwMDAwEgYSBBDAmgwaBnhpb24tMSAN").unwrap();

        // load proof from previously sent and proved email
        let proof_bz = Binary::from_base64("eyJwaV9hIjpbIjE5NjQ0MTcxMTA2NTUzNzQ3ODI3NDY4MTM2MTY3NTcwNTg0Njk2NTU2OTY4MjA4OTM3NDgzMDA4NzA0MzIwODkyMDg3OTc0Mjg0NjEzIiwiNTE3MTk5MjkwNzk4MjI5MDM1MDIxMzI5ODU0NzIwMTA1NTcxNjg3OTc5MzE3OTEwNzAwMzU3OTY2NDUwMTQwNzA1OTg1MDc1OTg0MCIsIjEiXSwicGlfYiI6W1siMTkzNzYwMjM4OTIyMzk4OTQ4OTIyMDAyMzQ0NzI2NjIxMjg5MTI4NTIzMjgyNzM3MDA4NzY0OTI5ODQxOTU3NTY5MDIyNjg4Njg1NTEiLCIxMzk4ODUyODMxOTQyNjM5MjM0MTU1MjcwOTc5Mzg2MDg4MzEwMjU5MjUwODY5MDQ2NzcwOTY4OTkzMzI4NTQ5NTUxNDQ3ODMzNDgxIl0sWyI4NzM1NTAxMzcyMjQyMzY5ODk5MTgxNzUyOTc3MDM3NTIzODQ3OTUzMTY5NTY0MjU4NDUwMDE0NTkwODMyMzA4NjUwOTE0NDExMTc3IiwiOTMzNTM1Nzc5NDExNjA1OTM5MTUxNTExNDMxMjMwMTM2OTUyNTAwODkwODQ0NjUwNTQ2MDQxNzgxNDc3MTU1NTczNzA5OTgzMDc2MiJdLFsiMSIsIjAiXV0sInBpX2MiOlsiMTMxODc4NzI3MzQ5MjA5OTAwNzQxMDAzMTkxNzI2MDg0Njc3NjU0MDY2MTgzMTc3NDI4ODQ0OTUyNjIxMzA2ODE5NzY1NzEyODA0MTIiLCIxNDUwNjU1NjY3MTIzMDgxOTUxMzc4MTExNTY1NDI5MjEzMzgzNDI3NTcxMzk2MjA3OTc2NTUyNjQ1MDI3MjU4NDIwMzEwNzYzMjY4NSIsIjEiXX0=").unwrap();
        let proof: SnarkJsProof = from_json(proof_bz).unwrap();

        // assign email salt from email used to prove
        let email_hash =
            Binary::from_base64("wjFSp5GIspdOPb0WLFyYfcZLQKt3k2E3thW+/dWuxw0=").unwrap();

        let dkim_domain = "google.com".to_string();
        let dkim_hash =
            Binary::from_base64("RjMpmQQPLhucCCEh0ouEDTeEkh4+IyOHCrVjy0R1iyo=").unwrap();

        // mock api for querying dkim module
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: CustomQuery {},
            custom_query_type: PhantomData::<Empty>,
        };
        init(deps.as_mut(), mock_env(), vkey).unwrap();

        // submit data for verification
        let res = query_verify(
            deps.as_ref(),
            proof,
            &tx_bytes,
            &email_hash,
            &dkim_domain,
            &dkim_hash,
        );
        match res {
            Ok(res) => {
                let verified: bool = from_json(res).unwrap();
                assert_eq!(verified, true);
            }
            Err(e) => panic!("error: {:?}", e),
        }
    }
}

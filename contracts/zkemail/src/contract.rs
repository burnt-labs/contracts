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
use base64::engine::general_purpose::{STANDARD_NO_PAD, STANDARD};
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
    let tx_input = calculate_tx_body_commitment(STANDARD.encode(tx_bytes).as_str());
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
pub mod tests {
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
            // return gmail.com dkim response
            let res: QueryDkimPubKeysResponse = QueryDkimPubKeysResponse {
                dkim_pub_keys: vec![
                    DkimPubKey { domain: "gmail.com".to_string(), pub_key: "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4zd3nfUoLHWFbfoPZzAb8bvjsFIIFsNypweLuPe4M".to_string(), poseidon_hash: Binary::from_base64("RjMpmQQPLhucCCEh0ouEDTeEkh4+IyOHCrVjy0R1iyo=").unwrap().into(), selector: "20230601".to_string(), version: 0, key_type: 0 }
                ],
                pagination: None,
            };
            SystemResult::Ok(ContractResult::Ok(res.to_bytes().unwrap().into()))
        }
    }
    
    #[test]
    fn verifying_zkemail_signature() {
        let vkey_64 = Binary::from_base64("eyJ2a19hbHBoYV8xIjpbIjIwNDkxMTkyODA1MzkwNDg1Mjk5MTUzMDA5NzczNTk0NTM0OTQwMTg5MjYxODY2MjI4NDQ3OTE4MDY4NjU4NDcxOTcwNDgxNzYzMDQyIiwiOTM4MzQ4NTM2MzA1MzI5MDIwMDkxODM0NzE1NjE1NzgzNjU2NjU2Mjk2Nzk5NDAzOTcxMjI3MzQ0OTkwMjYyMTI2NjE3ODU0NTk1OCIsIjEiXSwidmtfYmV0YV8yIjpbWyI2Mzc1NjE0MzUxNjg4NzI1MjA2NDAzOTQ4MjYyODY4OTYyNzkzNjI1NzQ0MDQzNzk0MzA1NzE1MjIyMDExNTI4NDU5NjU2NzM4NzMxIiwiNDI1MjgyMjg3ODc1ODMwMDg1OTEyMzg5Nzk4MTQ1MDU5MTM1MzUzMzA3MzQxMzE5Nzc3MTc2ODY1MTQ0MjY2NTc1MjI1OTM5NzEzMiJdLFsiMTA1MDUyNDI2MjYzNzAyNjIyNzc1NTI5MDEwODIwOTQzNTY2OTc0MDk4MzU2ODAyMjA1OTA5NzE4NzMxNzExNDAzNzEzMzEyMDY4NTYiLCIyMTg0NzAzNTEwNTUyODc0NTQwMzI4ODIzMjY5MTE0NzU4NDcyODE5MTE2MjczMjI5OTg2NTMzODM3NzE1OTY5MjM1MDA1OTEzNjY3OSJdLFsiMSIsIjAiXV0sInZrX2dhbW1hXzIiOltbIjEwODU3MDQ2OTk5MDIzMDU3MTM1OTQ0NTcwNzYyMjMyODI5NDgxMzcwNzU2MzU5NTc4NTE4MDg2OTkwNTE5OTkzMjg1NjU1ODUyNzgxIiwiMTE1NTk3MzIwMzI5ODYzODcxMDc5OTEwMDQwMjEzOTIyODU3ODM5MjU4MTI4NjE4MjExOTI1MzA5MTc0MDMxNTE0NTIzOTE4MDU2MzQiXSxbIjg0OTU2NTM5MjMxMjM0MzE0MTc2MDQ5NzMyNDc0ODkyNzI0Mzg0MTgxOTA1ODcyNjM2MDAxNDg3NzAyODA2NDkzMDY5NTgxMDE5MzAiLCI0MDgyMzY3ODc1ODYzNDMzNjgxMzMyMjAzNDAzMTQ1NDM1NTY4MzE2ODUxMzI3NTkzNDAxMjA4MTA1NzQxMDc2MjE0MTIwMDkzNTMxIl0sWyIxIiwiMCJdXSwidmtfZGVsdGFfMiI6W1siNTY4MTAwNjE2NDMwODI1MTk1MzAwMjExMzkyNTU4NTQ5MDE1OTM4MjM2NTQ1OTkwMDYxNjY5NTA2OTczODA5OTYxNjIyMTAwNzQ0MCIsIjE3ODQzNzI1MzIwMjMyODUyMzQ3ODgxMTI5MTI2MDUwMTY0MTg3ODU4NTMwODI2Nzg0MzAxNjI4NDE4MzM2ODg5NzI0MDQ3NTQ1MjYiXSxbIjEzMjA3MzcyNzAwMzc0OTUxMjE3OTcyNzM0MzA1OTg3OTg0MjExMjk5NTQxOTUzMjcyMTk1ODA0NDAzMDE5Mjk0Mjg1MDE1NDMyMjMwIiwiMTEyMDgwOTg4MTU5MzE0OTgyNTg1NjYzMzU5NTMyNTU3Njg5MTU3ODEwMjQ1Njg0MzE0NzI5NzMwMDM3NDAzNTEyNDIxNDUyNzQ4ODIiXSxbIjEiLCIwIl1dLCJJQyI6W1siMTM1NTI3OTYxNTkzMjE1MDA0NDY1NDIwOTIzMzM5MjU3ODk2MjcyMjE3NjM5OTU5MTcwNTc3MDUwMjUwMjQ3MzY4NDU3MjE5NDIzNTMiLCI5NjgwOTg1Mjg1MDQwMjMwNzUxNjQ0NDk1ODkzMjE1MzgyNjc2MDgxNTM1ODc2MzYzNTUzNjYwMzc0MTE1NTg4ODYyMDYwNDg3MDM0IiwiMSJdLFsiMTUyNTQ3NzY3NzI2MTA1MzMzMjc0ODg4MTQyODE5MzEyODc2NjU4OTM5NDMxNzc5NzczMTAxNjgwNTY3MjIzNjgyODE3Mjg2MTg2NjIiLCIxNDM2NzY1NzcwMTI0OTU0NzkxODUxMzY3NTUzMzA5MTI4NjM4MzM1ODE4NTY2Mjg2OTg4NzMxNjQ2NjgxMzA2MjE0MjY3NDQ5OTI2MCIsIjEiXSxbIjE0ODY2ODU5MTc3NzU4NjM1MDMwMDc5MjI2MzQxODYyNjAxMTEyOTgzNDg3ODM4NTExNjkwNTY3ODk1NTc0MDMwNjcyMjEyNDQxNjc2IiwiMTEzMTQ1NDIyOTM1MzM5NzMzMjg0MTY0NDE2NjMxNzYwMzIzOTk4NDU1MTkzMDA0NDY4OTQ0MDc1MjAwODM0MTIwMzY4ODgzNDI5OCIsIjEiXSxbIjE0NDUyOTAyNDgyODI4NTU4MjcwMjk4Mzc0NzM3NzA1NjU4MzUxMjcwNzg4NDI4MDE5NTg1MjQxMDI2OTAxMzMxMzY4ODU4NTI3OTQ4IiwiMTkwOTc5OTc3NzQ2MDY1MjM1MjcxNTg1Mjk1NTI2NDU5NjEyNTcxNTAxODkzNTczNTc2MDEwMzUwNzgxMTE2NDM5NTMzODUxNzEzODQiLCIxIl1dfQ==").unwrap();
        let vkey: SnarkJsVkey = from_json(vkey_64.clone()).unwrap();
        // build tx bytes to sign
        let tx_bytes = Binary::from_base64("CqIBCp8BChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEn8KP3hpb24xczN1YWU1MDUydTVnNzd3ZmxheHd5aDJ1dHg5OWU3aDZjNjJhNjI5M3J5cnh0Z3F2djhncXhlZ3J0axIreGlvbjFxYWYyeGZseDVqM2FndGx2cWs1dmhqcGV1aGw2ZzQ1aHhzaHdxahoPCgV1eGlvbhIGMTAwMDAwEhYSFAoOCgV1eGlvbhIFNjAwMDAQwJoMGgZ4aW9uLTEgCw==").unwrap();

        // load proof from previously sent and proved email
        let proof_bz = Binary::from_base64("eyJwaV9hIjpbIjg4MDE3Nzg4OTg5NTY3NTYxMDM5NDMwMTE1MzEzNjY0NjY4NTM3MzM1Nzk0MTI4MzI4ODM3NTQ0MjY4NzMwMDMyMjgzOTA0MDY2MzEiLCIxODA3NjgyNzE0MTg5Nzk1MTI3MTU4ODk2NTY1OTY5NjA5ODU0ODE0Njc1NTkyMjg5Nzk5Njk1MTc4NjA2NzE4NzcxOTA4OTg1NDQ5NSIsIjEiXSwicGlfYiI6W1siMTkzNDU4MDkxMzk5ODkxMzcwNTczMjQ0MTgzMjgzNDk3OTE0NjI1NDEwMjgyODczNjY1MzA1ODMwNTI1OTAzOTk4NDEzMDU4NDE0ODUiLCIyMTEzNDA5OTMxOTEwODcwMDQzMTQ2MDY2MTA3MTYzMDk4MzU4NDE1MjAxNTA5NjY2MjI0MjczOTE5Mjc2MTAzNTE4NzEyMTY4ODI5MiJdLFsiMTUwMDc2MzI4MjQ5MjQ3MjAyMjg5NTgzMjkwMDUxNzE2NzkwMjIyMTUzOTM1MzkyMDY3MTA4MDM0OTAwMjE3MDIzNjUxNTExNTk0MDEiLCIxMjczOTQyMTYzNzgxMjg3NDA3ODA5MDMzMzg0MTg0NTY1MzgwMjAyOTE0NTE2NDc3MDU4NDA4NDg2MDgwMDE0MDIxMDE1NTI3NjEyOCJdLFsiMSIsIjAiXV0sInBpX2MiOlsiNTcyNjkxMTk3Nzk1ODg2MjA0MTY3MjU3NTg1MjQxNDEwMTcyNTA4NTI5NzM2NDk4MDI4OTc5NzM1OTkyNjIyMjUzMjY4NTQ2MTgwNSIsIjQ0MzEyMjk1MDU2NTQ3MzcwMDkyMDM1NDMyNjM3NTk3MjM5NDk2MDYwMTg1MTY1MzczMTQ3NTk3MjkxNzA1NzU3NzEwOTQ0MDA3MTQiLCIxIl19").unwrap();
        let proof: SnarkJsProof = from_json(proof_bz).unwrap();

        // assign email salt from email used to prove
        let email_hash = Binary::from_base64("sAcYdn1nulpzJIM0RMaX6Vn5GPPGXuHxM//AfW7b7yU=").unwrap();

        let dkim_domain = "gmail.com".to_string();
        let dkim_hash =
            Binary::from_base64("iEeNSGFNAiTctrIgoVuE40DFz/ATm+ip5RBx3HfHqQ4=").unwrap();

        // mock api for querying dkim module
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: CustomQuery {},
            custom_query_type: PhantomData::<Empty>,
        };
        init(deps.as_mut(), mock_env(), vkey).unwrap();
        assert_eq!(
            Binary::from(deps.storage.get(b"vkey").unwrap()),
            vkey_64.clone()
        );

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

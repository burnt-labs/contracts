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

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use std::ops::Add;

    use crate::contract::{instantiate, sudo, AccountSudoMsg};
    use crate::msg::InstantiateMsg;
    use crate::AddAuthenticator;

    use super::*;
    use cosmos_sdk_proto::xion::v1::dkim::{DkimPubKey, QueryDkimPubKeysResponse};
    use serde::{Deserialize, Serialize};
    use zkemail::ark_verifier::SnarkJsVkey;
    use zkemail::contract::{query, init};
    use zkemail::msg::QueryMsg;
    use cosmwasm_std::{Binary, ContractResult, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery};
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi, MockStorage};
    use cosmos_sdk_proto::traits::MessageExt;

    #[derive(Serialize, Deserialize)]
    struct CustomQuery {}

    impl Querier for CustomQuery {
        fn raw_query(&self, _bin_request: &[u8]) -> QuerierResult {
            let msg_query = from_json::<QueryRequest<Empty>>(_bin_request).unwrap();
            let query_res = match msg_query {
                QueryRequest::Wasm(wasm_query) => {
                    match wasm_query {
                        WasmQuery::Smart { contract_addr: _, msg } => {
                            let mut deps = OwnedDeps {
                                storage: MockStorage::default(),
                                api: MockApi::default(),
                                querier: CustomQuery {},
                                custom_query_type: PhantomData::<Empty>,
                            };
                            let env = mock_env();
                            let vkey_bz = Binary::from_base64("eyJ2a19hbHBoYV8xIjpbIjIwNDkxMTkyODA1MzkwNDg1Mjk5MTUzMDA5NzczNTk0NTM0OTQwMTg5MjYxODY2MjI4NDQ3OTE4MDY4NjU4NDcxOTcwNDgxNzYzMDQyIiwiOTM4MzQ4NTM2MzA1MzI5MDIwMDkxODM0NzE1NjE1NzgzNjU2NjU2Mjk2Nzk5NDAzOTcxMjI3MzQ0OTkwMjYyMTI2NjE3ODU0NTk1OCIsIjEiXSwidmtfYmV0YV8yIjpbWyI2Mzc1NjE0MzUxNjg4NzI1MjA2NDAzOTQ4MjYyODY4OTYyNzkzNjI1NzQ0MDQzNzk0MzA1NzE1MjIyMDExNTI4NDU5NjU2NzM4NzMxIiwiNDI1MjgyMjg3ODc1ODMwMDg1OTEyMzg5Nzk4MTQ1MDU5MTM1MzUzMzA3MzQxMzE5Nzc3MTc2ODY1MTQ0MjY2NTc1MjI1OTM5NzEzMiJdLFsiMTA1MDUyNDI2MjYzNzAyNjIyNzc1NTI5MDEwODIwOTQzNTY2OTc0MDk4MzU2ODAyMjA1OTA5NzE4NzMxNzExNDAzNzEzMzEyMDY4NTYiLCIyMTg0NzAzNTEwNTUyODc0NTQwMzI4ODIzMjY5MTE0NzU4NDcyODE5MTE2MjczMjI5OTg2NTMzODM3NzE1OTY5MjM1MDA1OTEzNjY3OSJdLFsiMSIsIjAiXV0sInZrX2dhbW1hXzIiOltbIjEwODU3MDQ2OTk5MDIzMDU3MTM1OTQ0NTcwNzYyMjMyODI5NDgxMzcwNzU2MzU5NTc4NTE4MDg2OTkwNTE5OTkzMjg1NjU1ODUyNzgxIiwiMTE1NTk3MzIwMzI5ODYzODcxMDc5OTEwMDQwMjEzOTIyODU3ODM5MjU4MTI4NjE4MjExOTI1MzA5MTc0MDMxNTE0NTIzOTE4MDU2MzQiXSxbIjg0OTU2NTM5MjMxMjM0MzE0MTc2MDQ5NzMyNDc0ODkyNzI0Mzg0MTgxOTA1ODcyNjM2MDAxNDg3NzAyODA2NDkzMDY5NTgxMDE5MzAiLCI0MDgyMzY3ODc1ODYzNDMzNjgxMzMyMjAzNDAzMTQ1NDM1NTY4MzE2ODUxMzI3NTkzNDAxMjA4MTA1NzQxMDc2MjE0MTIwMDkzNTMxIl0sWyIxIiwiMCJdXSwidmtfZGVsdGFfMiI6W1siNTY4MTAwNjE2NDMwODI1MTk1MzAwMjExMzkyNTU4NTQ5MDE1OTM4MjM2NTQ1OTkwMDYxNjY5NTA2OTczODA5OTYxNjIyMTAwNzQ0MCIsIjE3ODQzNzI1MzIwMjMyODUyMzQ3ODgxMTI5MTI2MDUwMTY0MTg3ODU4NTMwODI2Nzg0MzAxNjI4NDE4MzM2ODg5NzI0MDQ3NTQ1MjYiXSxbIjEzMjA3MzcyNzAwMzc0OTUxMjE3OTcyNzM0MzA1OTg3OTg0MjExMjk5NTQxOTUzMjcyMTk1ODA0NDAzMDE5Mjk0Mjg1MDE1NDMyMjMwIiwiMTEyMDgwOTg4MTU5MzE0OTgyNTg1NjYzMzU5NTMyNTU3Njg5MTU3ODEwMjQ1Njg0MzE0NzI5NzMwMDM3NDAzNTEyNDIxNDUyNzQ4ODIiXSxbIjEiLCIwIl1dLCJJQyI6W1siMTM1NTI3OTYxNTkzMjE1MDA0NDY1NDIwOTIzMzM5MjU3ODk2MjcyMjE3NjM5OTU5MTcwNTc3MDUwMjUwMjQ3MzY4NDU3MjE5NDIzNTMiLCI5NjgwOTg1Mjg1MDQwMjMwNzUxNjQ0NDk1ODkzMjE1MzgyNjc2MDgxNTM1ODc2MzYzNTUzNjYwMzc0MTE1NTg4ODYyMDYwNDg3MDM0IiwiMSJdLFsiMTUyNTQ3NzY3NzI2MTA1MzMzMjc0ODg4MTQyODE5MzEyODc2NjU4OTM5NDMxNzc5NzczMTAxNjgwNTY3MjIzNjgyODE3Mjg2MTg2NjIiLCIxNDM2NzY1NzcwMTI0OTU0NzkxODUxMzY3NTUzMzA5MTI4NjM4MzM1ODE4NTY2Mjg2OTg4NzMxNjQ2NjgxMzA2MjE0MjY3NDQ5OTI2MCIsIjEiXSxbIjE0ODY2ODU5MTc3NzU4NjM1MDMwMDc5MjI2MzQxODYyNjAxMTEyOTgzNDg3ODM4NTExNjkwNTY3ODk1NTc0MDMwNjcyMjEyNDQxNjc2IiwiMTEzMTQ1NDIyOTM1MzM5NzMzMjg0MTY0NDE2NjMxNzYwMzIzOTk4NDU1MTkzMDA0NDY4OTQ0MDc1MjAwODM0MTIwMzY4ODgzNDI5OCIsIjEiXSxbIjE0NDUyOTAyNDgyODI4NTU4MjcwMjk4Mzc0NzM3NzA1NjU4MzUxMjcwNzg4NDI4MDE5NTg1MjQxMDI2OTAxMzMxMzY4ODU4NTI3OTQ4IiwiMTkwOTc5OTc3NzQ2MDY1MjM1MjcxNTg1Mjk1NTI2NDU5NjEyNTcxNTAxODkzNTczNTc2MDEwMzUwNzgxMTE2NDM5NTMzODUxNzEzODQiLCIxIl1dfQ==").unwrap();
                            let vkey = from_json::<SnarkJsVkey>(vkey_bz).unwrap();
                            init(deps.as_mut(), env.clone(), vkey).expect("could not create vkey");
                            let query_msg: QueryMsg = from_json(&msg).unwrap();
                            let query_res = query(deps.as_ref(), env, query_msg).expect("could not query verification contract");
                            return SystemResult::Ok(ContractResult::Ok(
                                query_res,
                            ));
                        }
                        _ => SystemResult::Err(SystemError::InvalidRequest { error: "unimplemented".to_string(), request: Binary::from([]) }),
                    }
                },
                QueryRequest::Grpc(_) => {
                    let res: QueryDkimPubKeysResponse = QueryDkimPubKeysResponse {
                        dkim_pub_keys: vec![
                            DkimPubKey { domain: "gmail.com".to_string(), pub_key: "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4zd3nfUoLHWFbfoPZzAb8bvjsFIIFsNypweLuPe4M".to_string(), poseidon_hash: Binary::from_base64("RjMpmQQPLhucCCEh0ouEDTeEkh4+IyOHCrVjy0R1iyo=").unwrap().into(), selector: "20230601".to_string(), version: 0, key_type: 0 }
                        ],
                        pagination: None,
                    };
                    SystemResult::Ok(ContractResult::Ok(res.to_bytes().unwrap().into()))
                }
                _ => SystemResult::Err(SystemError::InvalidRequest { error: "unimplemented".to_string(), request: Binary::from([]) }),
            };
            query_res
        }
    }
    
    #[test]
    fn test_verify() {
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: CustomQuery {},
            custom_query_type: PhantomData::<Empty>,
        };
        let tx_bytes = Binary::from_base64("CqIBCp8BChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEn8KP3hpb24xczN1YWU1MDUydTVnNzd3ZmxheHd5aDJ1dHg5OWU3aDZjNjJhNjI5M3J5cnh0Z3F2djhncXhlZ3J0axIreGlvbjFxYWYyeGZseDVqM2FndGx2cWs1dmhqcGV1aGw2ZzQ1aHhzaHdxahoPCgV1eGlvbhIGMTAwMDAwEhYSFAoOCgV1eGlvbhIFNjAwMDAQwJoMGhR4aW9uLWxvY2FsLXRlc3RuZXQtMSAK").unwrap();
        let sig_bytes = Binary::from_base64("AXsicHJvb2YiOnsicGlfYSI6WyI2ODA4NTk2OTc5NDA2NjE2MjU4MDA0OTEzODU4MjIwNzYyODI5NTQ0MDYyNDM0NTEzMjY3MzA2NzgzMDY2MTk3NjUwNzgwNTIzODI1IiwiNzgyNzIwODM1MDk0MTc2NDIxMTc0MTY3MDE0MTc0NzIxNzcxNTIyMTQ4MjcwMzc1ODE4MTE5MDM2ODA1MTQ3MjYxOTQ2NjUxMjgyIiwiMSJdLCJwaV9iIjpbWyIxMzc1NzA4MTkxMjUzNzkyMjgxNTUxNzQ4NjQ5NDQ5Nzg2OTc5NjU4NTI3MDc3OTk5MzIxMDk5MzE1MzQ4NDgyMTc0NDA2MjUxMjIwOSIsIjEyMjQ0MTkzMDY3NjQ0MDQ1MjU0MDYzMTUwMTc0Nzg4MjcwNzc1MjA5MzE2MjYxNTg0NDU0NjI5MzAxMzIzOTE5MTg5NDE2MjMwMDQwIl0sWyIxOTA1OTAxODM4MzE0OTM3OTQ2MTMzOTEwODc1NjI2NzE0NzQ3ODI4MTk0ODcyNTA0NDQ5ODM5ODMwNjkxMDkxODg1Nzk2OTYzOTEyMyIsIjE4MTA3MzYyMjA3MTY1OTA0ODU4MTI3ODk5NTMzODExMDc3MDQxMjMyMzA1MzIwNjU1NzE5NDkyMTAwNjc4OTQwNTE4ODE4MTQ4ODA5Il0sWyIxIiwiMCJdXSwicGlfYyI6WyI0MDA1MDc3MDcwODc0ODExNjg0MzIyNzU5MDI4MDI5NzE1MjgzMjc2MjAzMDQxMzc0MTA5OTc0Nzg5MDQ3MTgxMjUwMTgyNTA0ODgyIiwiMTEyODgyNzE5NjgwOTE4MzMwMjczODY0OTg4MzQyMjIxOTEwOTYyMjU5NjkzNDA4MDkzMTI5MDMwMDEzNjI2NzkzODYzMjQ1MTE4MDIiLCIxIl19LCJka2ltX2hhc2giOiJpRWVOU0dGTkFpVGN0cklnb1Z1RTQwREZ6L0FUbStpcDVSQngzSGZIcVE0PSJ9").unwrap();
        let email_hash = Binary::from_base64("sAcYdn1nulpzJIM0RMaX6Vn5GPPGXuHxM//AfW7b7yU=").unwrap();
        let dkim_domain = "gmail.com".to_string();

        let init_msg = InstantiateMsg {
            authenticator: AddAuthenticator::ZKEmail { id: 1, verification_contract: Addr::unchecked("verification contract"), email_hash, dkim_domain }
        };
        instantiate(deps.as_mut(), mock_env(), message_info(&Addr::unchecked("me"), &[]), init_msg).expect("could not instantiate");

        let sudo_msg = AccountSudoMsg::BeforeTx { msgs: vec![], tx_bytes, cred_bytes: Some(sig_bytes), simulate: false };
        let result = sudo( deps.as_mut(), mock_env(), sudo_msg );

        match result {
            Ok(_) => assert!(true),
            Err(err) => panic!("Verification failed: {:?}", err),
        }
    }
}
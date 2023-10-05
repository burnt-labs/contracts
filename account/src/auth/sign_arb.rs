use crate::auth::util;
use crate::auth::util::{derive_addr, sha256};
use crate::error::ContractResult;
use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::{Addr, Api};

pub fn verify(
    api: &dyn Api,
    msg_bytes: &[u8],
    sig_bytes: &[u8],
    pubkey: &[u8],
) -> ContractResult<bool> {
    let signer_s = derive_addr(util::CHAIN_BECH_PREFIX, pubkey)?;
    let signer = api.addr_validate(signer_s.as_str())?;

    let envelope_hash = wrap_message(msg_bytes, signer);

    let verification = api.secp256k1_verify(envelope_hash.as_slice(), sig_bytes, pubkey)?;
    Ok(verification)
}

fn wrap_message(msg_bytes: &[u8], signer: Addr) -> Vec<u8> {
    let msg_b64 = general_purpose::STANDARD.encode(msg_bytes);
    // format the msg in the style of ADR-036 SignArbitrary
    let  envelope = format!("{{\"account_number\":\"0\",\"chain_id\":\"\",\"fee\":{{\"amount\":[],\"gas\":\"0\"}},\"memo\":\"\",\"msgs\":[{{\"type\":\"sign/MsgSignData\",\"value\":{{\"data\":\"{}\",\"signer\":\"{}\"}}}}],\"sequence\":\"0\"}}", msg_b64.as_str(), signer.as_str());

    return sha256(envelope.to_string().as_bytes());
}

#[cfg(test)]
mod tests {
    use crate::auth::sign_arb::wrap_message;
    use crate::auth::{util, AddAuthenticator, Authenticator};
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use base64::engine::general_purpose;
    use base64::{engine, Engine};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, Binary};
    use cosmwasm_std::{Addr, Response};
    use cosmwasm_std::{Api, Attribute};

    fn wrap_message_string(msg_bytes: &[u8], signer: Addr) -> String {
        let msg_b64 = general_purpose::STANDARD.encode(msg_bytes);
        // format the msg in the style of ADR-036 SignArbitrary
        let  envelope = format!("{{\"account_number\":\"0\",\"chain_id\":\"\",\"fee\":{{\"amount\":[],\"gas\":\"0\"}},\"memo\":\"\",\"msgs\":[{{\"type\":\"sign/MsgSignData\",\"value\":{{\"data\":\"{}\",\"signer\":\"{}\"}}}}],\"sequence\":\"0\"}}", msg_b64.as_str(), signer.as_str());

        return envelope;
    }

    #[test]
    fn test_derive_addr() {
        let pub_key = "AxVQixKMvKkMWMgEBn5E+QjXxFLLiOUNs3EG3vvsgaGs";
        let pub_key_bytes = general_purpose::STANDARD.decode(pub_key).unwrap();

        let deps = mock_dependencies();
        let addr = util::derive_addr("osmo", pub_key_bytes.as_slice()).unwrap();

        let valid_addr = deps.api.addr_validate(addr.as_str()).unwrap();

        assert_eq!(
            "osmo1ee3y7m9kjn8xgqwryxmskv6ttnkj39z9w0fctn",
            valid_addr.as_str()
        );
    }

    #[test]
    fn test_verify_sign_arb() {
        let pubkey = "AxVQixKMvKkMWMgEBn5E+QjXxFLLiOUNs3EG3vvsgaGs";
        let pubkey_bytes = general_purpose::STANDARD.decode(pubkey).unwrap();

        let deps = mock_dependencies();
        let signer_s = util::derive_addr("xion", pubkey_bytes.as_slice()).unwrap();
        let signer = deps.api.addr_validate(signer_s.as_str()).unwrap();

        assert_eq!(
            "xion1ee3y7m9kjn8xgqwryxmskv6ttnkj39z9yaq2t2",
            signer.as_str()
        );

        let test_msg = "WooHoo";

        let test_msg_b64 = general_purpose::STANDARD.encode(test_msg);
        assert_eq!("V29vSG9v", test_msg_b64);

        let env_hash = wrap_message(test_msg.as_bytes(), signer);

        let expected_signature = "E5AKzlomNEYUjtYbdC8Boqlg2UIcHUL3tOq1e9CEcmlBMnONpPaAFQIZzJLIT6Jx87ViSTW58LJwGdFQqh0otA==";
        let expected_sig_bytes = general_purpose::STANDARD
            .decode(expected_signature)
            .unwrap();
        let verification = deps
            .api
            .secp256k1_verify(
                env_hash.as_slice(),
                expected_sig_bytes.as_slice(),
                pubkey_bytes.as_slice(),
            )
            .unwrap();
        assert!(verification)
    }

    #[test]
    fn test_init() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let info = mock_info("creator", &[]);

        let id: u8 = 1;
        let pubkey = "AxVQixKMvKkMWMgEBn5E+QjXxFLLiOUNs3EG3vvsgaGs";
        let pubkey_bytes = engine::general_purpose::STANDARD.decode(pubkey).unwrap();
        let expected_signature = "E5AKzlomNEYUjtYbdC8Boqlg2UIcHUL3tOq1e9CEcmlBMnONpPaAFQIZzJLIT6Jx87ViSTW58LJwGdFQqh0otA==";
        let expected_sig_bytes = engine::general_purpose::STANDARD
            .decode(expected_signature)
            .unwrap();

        let test_msg = "WooHoo";

        let signer_s = util::derive_addr("xion", pubkey_bytes.as_slice()).unwrap();
        let signer = deps.api.addr_validate(signer_s.as_str()).unwrap();
        let test_msg_b64 = general_purpose::STANDARD.encode(test_msg);
        assert_eq!("V29vSG9v", test_msg_b64);

        let env_hash = wrap_message_string(test_msg.as_bytes(), signer);
        env.contract.address = Addr::unchecked(env_hash);

        let authenticator = Authenticator::Secp256K1 {
            pubkey: pubkey_bytes.into(),
        };
        let signature: Binary = expected_sig_bytes.into();

        let msg = InstantiateMsg {
            id,
            authenticator: authenticator.clone(),
            signature: signature.clone(),
        };
        let res: Response = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let events = res.events;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].ty, "create_abstract_account");
        assert_eq!(
            events[0].attributes,
            vec![
                Attribute::new("contract_address", env.contract.address.to_string()),
                Attribute::new(
                    "authenticator",
                    serde_json::to_string(&authenticator).unwrap()
                ),
            ]
        );

        let query_response = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::AuthenticatorByID { id },
        )
        .unwrap();

        match from_binary::<Binary>(&query_response) {
            Ok(retrieved_authenticator) => {
                match from_binary::<Authenticator>(&retrieved_authenticator) {
                    Ok(retrieved_authenticator) => {
                        assert_eq!(retrieved_authenticator, authenticator)
                    }
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        };
    }

    #[test]
    fn test_add_remove_auth_method_events() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        let mut info = mock_info("creator", &[]);

        let id: u8 = 1;
        let pubkey = "AxVQixKMvKkMWMgEBn5E+QjXxFLLiOUNs3EG3vvsgaGs";
        let pubkey_bytes = engine::general_purpose::STANDARD.decode(pubkey).unwrap();
        let expected_signature = "E5AKzlomNEYUjtYbdC8Boqlg2UIcHUL3tOq1e9CEcmlBMnONpPaAFQIZzJLIT6Jx87ViSTW58LJwGdFQqh0otA==";
        let expected_sig_bytes = engine::general_purpose::STANDARD
            .decode(expected_signature)
            .unwrap();

        let test_msg = "WooHoo";

        let signer_s = util::derive_addr("xion", pubkey_bytes.as_slice()).unwrap();
        let signer = deps.api.addr_validate(signer_s.as_str()).unwrap();
        let test_msg_b64 = general_purpose::STANDARD.encode(test_msg);
        assert_eq!("V29vSG9v", test_msg_b64);

        let env_hash = wrap_message_string(test_msg.as_bytes(), signer);
        env.contract.address = Addr::unchecked(env_hash.clone());
        info.sender = Addr::unchecked(env_hash);

        let signature: Binary = expected_sig_bytes.into();

        let mut add_msg = ExecuteMsg::AddAuthMethod {
            add_authenticator: AddAuthenticator::Secp256K1 {
                id,
                pubkey: pubkey_bytes.clone().into(),
                signature: signature.clone(),
            },
        };
        let res: Response = execute(deps.as_mut(), env.clone(), info.clone(), add_msg).unwrap();

        let events = res.events;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].ty, "add_auth_method");
        assert_eq!(
            events[0].attributes,
            vec![
                Attribute::new("contract_address", env.contract.address.to_string()),
                Attribute::new(
                    "authenticator",
                    serde_json::to_string(&AddAuthenticator::Secp256K1 {
                        pubkey: pubkey_bytes.clone().into(),
                        id,
                        signature: signature.clone()
                    })
                    .unwrap()
                ),
            ]
        );

        let query_response = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::AuthenticatorByID { id },
        )
        .unwrap();

        match from_binary::<Binary>(&query_response) {
            Ok(retrieved_authenticator) => {
                match from_binary::<Authenticator>(&retrieved_authenticator) {
                    Ok(retrieved_authenticator) => {
                        assert_eq!(
                            retrieved_authenticator,
                            Authenticator::Secp256K1 {
                                pubkey: pubkey_bytes.clone().into()
                            }
                        )
                    }
                    Err(e) => panic!("Unexpected error: {:?}", e),
                }
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        let remove_msg = ExecuteMsg::RemoveAuthMethod { id: 1 };
        // make sure we cannot remove all authenticators
        execute(deps.as_mut(), env.clone(), info.clone(), remove_msg.clone())
            .expect_err("MinimumAuthenticatorCount");
        // add another authenticator
        add_msg = ExecuteMsg::AddAuthMethod {
            add_authenticator: AddAuthenticator::Secp256K1 {
                id: 2,
                pubkey: pubkey_bytes.clone().into(),
                signature: signature.clone(),
            },
        };
        execute(deps.as_mut(), env.clone(), info.clone(), add_msg).expect("AddAuthMethod");
        // remove the first authenticator
        let res = execute(deps.as_mut(), env.clone(), info.clone(), remove_msg).unwrap();

        let events = res.events;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].ty, "remove_auth_method");
        assert_eq!(
            events[0].attributes,
            vec![
                Attribute::new("contract_address", env.contract.address.to_string()),
                Attribute::new("authenticator_id", 1.to_string()),
            ]
        );

        query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::AuthenticatorByID { id },
        )
        .expect_err("AuthenticatorNotFound");
    }
}

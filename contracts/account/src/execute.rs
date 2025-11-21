use std::borrow::BorrowMut;

use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, Event, Order, Response};

use crate::auth::{jwt, passkey, AddAuthenticator, Authenticator};
use crate::{
    error::{ContractError, ContractResult},
    state::AUTHENTICATORS,
};


pub fn init(
    deps: DepsMut,
    env: Env,
    add_authenticator: &mut AddAuthenticator,
) -> ContractResult<Response> {
    add_auth_method(deps, &env, add_authenticator)?;

    Ok(
        Response::new().add_event(Event::new("create_abstract_account").add_attributes(vec![
            ("contract_address", env.contract.address.to_string()),
            ("authenticator", serde_json::to_string(&add_authenticator)?),
            ("authenticator_id", add_authenticator.get_id().to_string()),
        ])),
    )
}

pub fn before_tx(
    deps: Deps,
    env: &Env,
    tx_bytes: &Binary,
    cred_bytes: Option<&Binary>,
    simulate: bool,
) -> ContractResult<Response> {
    if !simulate {
        let cred_bytes = cred_bytes.ok_or(ContractError::EmptySignature)?;
        // currently, the minimum size of a signature by any auth method is 64 bytes
        // this may change in the future, and this check will need to be re-evaluated.
        //
        // checking the cred_bytes are at least 1 + 64 bytes long
        if cred_bytes.len() < 65 {
            return Err(ContractError::ShortSignature);
        }

        // the first byte of the signature is the index of the authenticator
        let cred_index: u8 = match cred_bytes.first() {
            None => return Err(ContractError::InvalidSignature),
            Some(i) => *i,
        };
        // retrieve the authenticator by index, or error
        let authenticator = AUTHENTICATORS.load(deps.storage, cred_index)?;

        let sig_bytes = &Binary::from(&cred_bytes.as_slice()[1..]);

        match authenticator {
            Authenticator::Secp256K1 { .. }
            | Authenticator::Ed25519 { .. }
            | Authenticator::Secp256R1 { .. } => {
                if sig_bytes.len() != 64 {
                    return Err(ContractError::ShortSignature);
                }
            }
            Authenticator::EthWallet { .. } => {
                if sig_bytes.len() != 65 {
                    return Err(ContractError::ShortSignature);
                }
            }
            Authenticator::Jwt { .. } => {
                // todo: figure out if there are minimum checks for JWTs
            }
            Authenticator::Passkey { .. } => {
                // todo: figure out if there are minimum checks for passkeys
            }
            Authenticator::ZKEmail { .. } => {
                // todo: verify that this minimum is as high as possible
                if sig_bytes.len() < 700 {
                    return Err(ContractError::ShortSignature);
                }
            }
        }

        return match authenticator.verify(deps, env, tx_bytes, sig_bytes)? {
            true => Ok(Response::new().add_attribute("method", "before_tx")),
            false => Err(ContractError::InvalidSignature),
        };
    }

    Ok(Response::new().add_attribute("method", "before_tx"))
}

pub fn after_tx() -> ContractResult<Response> {
    Ok(Response::new().add_attribute("method", "after_tx"))
}

pub fn add_auth_method(
    deps: DepsMut,
    env: &Env,
    add_authenticator: &mut AddAuthenticator,
) -> ContractResult<Response> {
    match add_authenticator.borrow_mut() {
        AddAuthenticator::Secp256K1 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Secp256K1 {
                pubkey: (*pubkey).clone(),
            };

            if !auth.verify(
                deps.as_ref(),
                env,
                &Binary::from(env.contract.address.as_bytes()),
                signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, *id, &auth)?;
                Ok(())
            }
        }
        AddAuthenticator::Ed25519 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Ed25519 {
                pubkey: (*pubkey).clone(),
            };

            if !auth.verify(
                deps.as_ref(),
                env,
                &Binary::from(env.contract.address.as_bytes()),
                signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, *id, &auth)?;
                Ok(())
            }
        }
        AddAuthenticator::EthWallet {
            id,
            address,
            signature,
        } => {
            let auth = Authenticator::EthWallet {
                address: (*address).clone(),
            };

            if !auth.verify(
                deps.as_ref(),
                env,
                &Binary::from(env.contract.address.as_bytes()),
                signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, *id, &auth)?;
                Ok(())
            }
        }
        AddAuthenticator::Jwt {
            id,
            aud,
            sub,
            token,
        } => {
            let auth = Authenticator::Jwt {
                aud: (*aud).clone(),
                sub: (*sub).clone(),
            };

            jwt::verify(
                deps.as_ref(),
                &Binary::from(env.contract.address.as_bytes()).to_vec(),
                token,
                aud,
                sub,
            )?;

            save_authenticator(deps, *id, &auth)?;
            Ok(())
        }
        AddAuthenticator::Secp256R1 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Secp256R1 {
                pubkey: (*pubkey).clone(),
            };

            if !auth.verify(
                deps.as_ref(),
                env,
                &Binary::from(env.contract.address.as_bytes()),
                signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, *id, &auth)?;

                Ok(())
            }
        }
        AddAuthenticator::Passkey {
            id,
            url,
            credential,
        } => {
            let passkey = passkey::register(
                deps.as_ref(),
                env.contract.address.clone(),
                (*url).clone(),
                (*credential).clone(),
            )?;

            let auth = Authenticator::Passkey {
                url: (*url).clone(),
                passkey: passkey.clone(),
            };
            save_authenticator(deps, *id, &auth)?;
            // we replace the sent credential with the passkey for indexers and other
            // observers to see
            *(credential) = passkey;
            Ok(())
        }
        AddAuthenticator::ZKEmail {
            id,
            email_salt,
            allowed_email_hosts,
            signature,
        } => {
            // Validate that at least one email host is provided
            if allowed_email_hosts.is_empty() {
                return Err(ContractError::NoAllowedEmailHosts);
            }

            let auth = Authenticator::ZKEmail {
                email_salt: (*email_salt).clone(),
                allowed_email_hosts: allowed_email_hosts.clone(),
            };
            if !auth.verify(
                deps.as_ref(),
                env,
                &Binary::from(env.contract.address.as_bytes()),
                signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, *id, &auth)?;
                Ok(())
            }
        }
    }?;
    Ok(
        Response::new().add_event(Event::new("add_auth_method").add_attributes(vec![
            ("contract_address", env.contract.address.clone().to_string()),
            ("authenticator", serde_json::to_string(&add_authenticator)?),
        ])),
    )
}

pub fn save_authenticator(
    deps: DepsMut,
    id: u8,
    authenticator: &Authenticator,
) -> ContractResult<()> {
    if AUTHENTICATORS.has(deps.storage, id) {
        return Err(ContractError::OverridingIndex { index: id });
    }

    AUTHENTICATORS.save(deps.storage, id, authenticator)?;
    Ok(())
}

pub fn remove_auth_method(deps: DepsMut, env: Env, id: u8) -> ContractResult<Response> {
    // Ensure there is more than one authenticator before removing
    if AUTHENTICATORS
        .keys(deps.storage, None, None, Order::Ascending)
        .count()
        <= 1
    {
        return Err(ContractError::MinimumAuthenticatorCount);
    }

    // Remove the authenticator
    AUTHENTICATORS.remove(deps.storage, id);

    Ok(
        Response::new().add_event(Event::new("remove_auth_method").add_attributes(vec![
            ("contract_address", env.contract.address.to_string()),
            ("authenticator_id", id.to_string()),
        ])),
    )
}

const MAX_SIZE: usize = 1024;
pub fn emit(env: Env, data: String) -> ContractResult<Response> {
    if data.len() > MAX_SIZE {
        Err(ContractError::EmissionSizeExceeded)
    } else {
        let emit_event = Event::new("account_emit")
            .add_attribute("address", env.contract.address)
            .add_attribute("data", data);
        Ok(Response::new().add_event(emit_event))
    }
}

pub fn assert_self(sender: &Addr, contract: &Addr) -> ContractResult<()> {
    if sender != contract {
        return Err(ContractError::Unauthorized);
    }

    Ok(())
}

pub fn update_allowed_email_hosts(
    deps: DepsMut,
    env: Env,
    id: u8,
    allowed_email_hosts: Vec<String>,
) -> ContractResult<Response> {
    // Validate that at least one email host is provided
    if allowed_email_hosts.is_empty() {
        return Err(ContractError::NoAllowedEmailHosts);
    }

    // Load the authenticator
    let authenticator = AUTHENTICATORS.load(deps.storage, id)?;

    // Ensure it's a ZKEmail authenticator
    match authenticator {
        Authenticator::ZKEmail { email_salt, .. } => {
            // Update the authenticator with new allowed_email_hosts
            let updated_auth = Authenticator::ZKEmail {
                email_salt,
                allowed_email_hosts: allowed_email_hosts.clone(),
            };
            
            AUTHENTICATORS.save(deps.storage, id, &updated_auth)?;

            Ok(Response::new().add_event(
                Event::new("update_allowed_email_hosts").add_attributes(vec![
                    ("contract_address", env.contract.address.to_string()),
                    ("authenticator_id", id.to_string()),
                    ("allowed_email_hosts", serde_json::to_string(&allowed_email_hosts)?),
                ]),
            ))
        }
        _ => Err(ContractError::UnsupportedAuthenticatorOperation),
    }
}

pub fn add_allowed_email_host(
    deps: DepsMut,
    env: Env,
    id: u8,
    email_host: String,
) -> ContractResult<Response> {
    // Load the authenticator
    let authenticator = AUTHENTICATORS.load(deps.storage, id)?;

    // Ensure it's a ZKEmail authenticator
    match authenticator {
        Authenticator::ZKEmail {
            email_salt,
            mut allowed_email_hosts,
        } => {
            // Check if the email host already exists
            if allowed_email_hosts.contains(&email_host) {
                return Ok(Response::new().add_event(
                    Event::new("add_allowed_email_host").add_attributes(vec![
                        ("contract_address", env.contract.address.to_string()),
                        ("authenticator_id", id.to_string()),
                        ("email_host", email_host),
                        ("status", "already_exists".to_string()),
                    ]),
                ));
            }

            // Add the new email host
            allowed_email_hosts.push(email_host.clone());

            // Update the authenticator
            let updated_auth = Authenticator::ZKEmail {
                email_salt,
                allowed_email_hosts,
            };

            AUTHENTICATORS.save(deps.storage, id, &updated_auth)?;

            Ok(Response::new().add_event(
                Event::new("add_allowed_email_host").add_attributes(vec![
                    ("contract_address", env.contract.address.to_string()),
                    ("authenticator_id", id.to_string()),
                    ("email_host", email_host),
                    ("status", "added".to_string()),
                ]),
            ))
        }
        _ => Err(ContractError::UnsupportedAuthenticatorOperation),
    }
}

pub fn remove_allowed_email_host(
    deps: DepsMut,
    env: Env,
    id: u8,
    email_host: String,
) -> ContractResult<Response> {
    // Load the authenticator
    let authenticator = AUTHENTICATORS.load(deps.storage, id)?;

    // Ensure it's a ZKEmail authenticator
    match authenticator {
        Authenticator::ZKEmail {
            email_salt,
            mut allowed_email_hosts,
        } => {
            // Ensure at least one email host remains after removal
            if allowed_email_hosts.len() <= 1 {
                return Err(ContractError::NoAllowedEmailHosts);
            }

            // Remove the email host
            allowed_email_hosts.retain(|host| host != &email_host);

            // Update the authenticator
            let updated_auth = Authenticator::ZKEmail {
                email_salt,
                allowed_email_hosts,
            };

            AUTHENTICATORS.save(deps.storage, id, &updated_auth)?;

            Ok(Response::new().add_event(
                Event::new("remove_allowed_email_host").add_attributes(vec![
                    ("contract_address", env.contract.address.to_string()),
                    ("authenticator_id", id.to_string()),
                    ("email_host", email_host),
                    ("status", "removed".to_string()),
                ]),
            ))
        }
        _ => Err(ContractError::UnsupportedAuthenticatorOperation),
    }
}

#[cfg(test)]
pub mod tests {
    use base64::{engine::general_purpose, Engine as _};
    use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{Binary, CustomQuery, OwnedDeps};
    use serde::{Deserialize, Serialize};

    use crate::auth::Authenticator;
    use crate::execute::before_tx;
    use crate::state::AUTHENTICATORS;
    use cosmwasm_std::QueryRequest::Custom;

    use cosmos_sdk_proto::xion::v1::{
        QueryWebAuthNVerifyAuthenticateRequest, QueryWebAuthNVerifyRegisterRequest,
        QueryWebAuthNVerifyRegisterResponse,
    };

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
    #[serde(rename_all = "snake_case")]
    pub enum XionCustomQuery {
        Verify(QueryWebAuthNVerifyRegisterRequest),
        Authenticate(QueryWebAuthNVerifyAuthenticateRequest),
    }

    impl CustomQuery for XionCustomQuery {}

    #[test]
    fn test_before_tx() {
        let auth_id = 0;
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default().with_prefix("xion"),
            querier: MockQuerier::<XionCustomQuery>::new(&[]),
            custom_query_type: std::marker::PhantomData,
        };
        let env = mock_env();

        let pubkey = "Ayrlj6q3WWs91p45LVKwI8JyfMYNmWMrcDinLNEdWYE4";
        let pubkey_bytes = general_purpose::STANDARD.decode(pubkey).unwrap();
        let auth = Authenticator::Secp256K1 {
            pubkey: Binary::from(pubkey_bytes),
        };

        let signature = "UDerMpp4QzGxjuu3uTmqoOdPrmRnwiOf6BOlL5xG2pAEx+gS8DV3HwBzrb+QRIVyKVc3D7RYMOAlRFRkpVANDA==";
        let sig_arr = general_purpose::STANDARD.decode(signature).unwrap();

        // The index of the first authenticator is 0.
        let cred_index = vec![0u8];

        let mut new_vec = Vec::new();
        new_vec.extend_from_slice(&cred_index);
        new_vec.extend_from_slice(&sig_arr);

        AUTHENTICATORS
            .save(deps.as_mut().storage, auth_id, &auth)
            .unwrap();

        let sig_bytes = Binary::from(new_vec);
        let tx_bytes = Binary::from(general_purpose::STANDARD.decode("Cp0BCpoBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEnoKP3hpb24xbTZ2aDIwcHM3NW0ybjZxeHdwandmOGZzM2t4dzc1enN5M3YycnllaGQ5c3BtbnUwcTlyc2g0NnljeRIreGlvbjFlMmZ1d2UzdWhxOHpkOW5ra2s4NzZuYXdyd2R1bGd2NDYwdnpnNxoKCgV1eGlvbhIBMRJTCksKQwodL2Fic3RyYWN0YWNjb3VudC52MS5OaWxQdWJLZXkSIgog3pl1PDD1NqnoBnBk5J0wjYzvUFAkWKGTN2lgHc+PAUcSBAoCCAESBBDgpxIaFHhpb24tbG9jYWwtdGVzdG5ldC0xIAg=").unwrap());

        before_tx(deps.as_ref(), &env, &tx_bytes, Some(&sig_bytes), false).unwrap();
    }

    #[test]
    pub fn test_custom_querier() {
        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: MockQuerier::<XionCustomQuery>::new(&[]),
            custom_query_type: core::marker::PhantomData::<XionCustomQuery>,
        };

        deps.querier = deps.querier.with_custom_handler(|query| match query {
            XionCustomQuery::Verify(data) => {
                assert_eq!(data.addr, "mock_address");
                assert_eq!(data.challenge, "mock_challenge");
                assert_eq!(data.rp, "mock_rp");
                assert_eq!(data.data, vec![0u8]);

                cosmwasm_std::SystemResult::Ok(cosmwasm_std::ContractResult::Ok(
                    serde_json::to_vec(&QueryWebAuthNVerifyRegisterResponse {
                        credential: Binary::from("true".as_bytes()).into(),
                    })
                    .unwrap()
                    .into(),
                ))
            }
            XionCustomQuery::Authenticate(_) => todo!(),
        });

        let query_msg = XionCustomQuery::Verify(QueryWebAuthNVerifyRegisterRequest {
            addr: "mock_address".to_string(),
            challenge: "mock_challenge".to_string(),
            rp: "mock_rp".to_string(),
            data: vec![0u8],
        });
        let query_response = deps
            .as_ref()
            .querier
            .query::<QueryWebAuthNVerifyRegisterResponse>(&Custom(query_msg));
        assert!(query_response.is_ok());

        assert_eq!(
            query_response.unwrap().credential,
            Binary::from("true".as_bytes())
        );
    }

    #[test]
    fn test_add_zkemail_authenticator_invalid_signature() {
        use crate::auth::AddAuthenticator;
        use crate::execute::add_auth_method;

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default().with_prefix("xion"),
            querier: MockQuerier::<XionCustomQuery>::new(&[]),
            custom_query_type: std::marker::PhantomData,
        };
        let env = mock_env();

        // Create an invalid signature (insufficient public outputs)
        let invalid_signature_json = r#"{
            "proof": {
                "pi_a": ["1", "2", "3"],
                "pi_b": [["4", "5"], ["6", "7"], ["8", "9"]],
                "pi_c": ["10", "11", "12"],
                "protocol": "groth16"
            },
            "publicInputs": ["1", "2", "3"]
        }"#;

        let signature_binary = Binary::from(invalid_signature_json.as_bytes());

        let mut add_authenticator = AddAuthenticator::ZKEmail {
            id: 1,
            email_salt: "test_email_salt".to_string(),
            allowed_email_hosts: vec!["example.com".to_string()],
            signature: signature_binary,
        };

        // Call add_auth_method - should fail due to insufficient public outputs
        let result = add_auth_method(deps.as_mut(), &env, &mut add_authenticator);
        
        // Verify the result is an error
        assert!(result.is_err());
        
        // Verify the authenticator was not saved
        assert!(!AUTHENTICATORS.has(deps.as_ref().storage, 1));
    }

    #[test]
    fn test_allowed_email_hosts_operations() {
        use crate::auth::Authenticator;
        use crate::error::ContractError;
        use crate::execute::{add_allowed_email_host, remove_allowed_email_host, update_allowed_email_hosts};

        let mut deps = OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default().with_prefix("xion"),
            querier: MockQuerier::<XionCustomQuery>::new(&[]),
            custom_query_type: std::marker::PhantomData,
        };
        let env = mock_env();
        let auth_id = 1u8;

        // Setup: Create a ZKEmail authenticator with initial email hosts
        let initial_authenticator = Authenticator::ZKEmail {
            email_salt: "test_salt".to_string(),
            allowed_email_hosts: vec!["example.com".to_string(), "test.com".to_string()],
        };
        AUTHENTICATORS
            .save(deps.as_mut().storage, auth_id, &initial_authenticator)
            .unwrap();

        // Test 1: Add a new email host
        let result = add_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "newhost.com".to_string(),
        );
        assert!(result.is_ok());
        
        // Verify the host was added
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts.len(), 3);
                assert!(allowed_email_hosts.contains(&"newhost.com".to_string()));
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 2: Try to add a duplicate email host
        let result = add_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "newhost.com".to_string(),
        );
        assert!(result.is_ok()); // Should succeed but not add duplicate
        
        // Verify no duplicate was added
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts.len(), 3); // Still 3, not 4
                assert_eq!(
                    allowed_email_hosts.iter().filter(|h| *h == "newhost.com").count(),
                    1
                );
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 3: Remove an email host
        let result = remove_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "newhost.com".to_string(),
        );
        assert!(result.is_ok());
        
        // Verify the host was removed
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts.len(), 2);
                assert!(!allowed_email_hosts.contains(&"newhost.com".to_string()));
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 4: Try to remove the second-to-last email host (should succeed)
        let result = remove_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "example.com".to_string(),
        );
        assert!(result.is_ok());
        
        // Verify only one host remains
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts.len(), 1);
                assert_eq!(allowed_email_hosts[0], "test.com");
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 5: Try to remove the last email host (should fail)
        let result = remove_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "test.com".to_string(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ContractError::NoAllowedEmailHosts);
        
        // Verify the host was not removed
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts.len(), 1);
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 6: Update allowed email hosts with a new list
        let new_hosts = vec![
            "updated1.com".to_string(),
            "updated2.com".to_string(),
            "updated3.com".to_string(),
        ];
        let result = update_allowed_email_hosts(
            deps.as_mut(),
            env.clone(),
            auth_id,
            new_hosts.clone(),
        );
        assert!(result.is_ok());
        
        // Verify the hosts were updated
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts, new_hosts);
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 7: Try to update with an empty list (should fail)
        let result = update_allowed_email_hosts(
            deps.as_mut(),
            env.clone(),
            auth_id,
            vec![],
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ContractError::NoAllowedEmailHosts);
        
        // Verify hosts were not changed
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts, new_hosts);
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 8: Try operations on a non-existent authenticator
        let result = add_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            99u8, // Non-existent ID
            "test.com".to_string(),
        );
        assert!(result.is_err());
        
        // Test 9: Try operations on a non-ZKEmail authenticator
        let secp_auth = Authenticator::Secp256K1 {
            pubkey: Binary::from(vec![1, 2, 3]),
        };
        AUTHENTICATORS
            .save(deps.as_mut().storage, 2u8, &secp_auth)
            .unwrap();
        
        let result = add_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            2u8,
            "test.com".to_string(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ContractError::UnsupportedAuthenticatorOperation);
        
        let result = remove_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            2u8,
            "test.com".to_string(),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ContractError::UnsupportedAuthenticatorOperation);
        
        let result = update_allowed_email_hosts(
            deps.as_mut(),
            env.clone(),
            2u8,
            vec!["test.com".to_string()],
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ContractError::UnsupportedAuthenticatorOperation);

        // Test 10: Try to remove a host that doesn't exist
        let result = remove_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "nonexistent.com".to_string(),
        );
        // This should succeed but not change anything
        assert!(result.is_ok());
        
        // Verify the hosts remain unchanged
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts, new_hosts);
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 11: Update with a single host (edge case - minimum valid)
        let result = update_allowed_email_hosts(
            deps.as_mut(),
            env.clone(),
            auth_id,
            vec!["single.com".to_string()],
        );
        assert!(result.is_ok());
        
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts.len(), 1);
                assert_eq!(allowed_email_hosts[0], "single.com");
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 12: Add multiple hosts one by one
        let result1 = add_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "multi1.com".to_string(),
        );
        assert!(result1.is_ok());
        
        let result2 = add_allowed_email_host(
            deps.as_mut(),
            env.clone(),
            auth_id,
            "multi2.com".to_string(),
        );
        assert!(result2.is_ok());
        
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { allowed_email_hosts, .. } => {
                assert_eq!(allowed_email_hosts.len(), 3);
                assert!(allowed_email_hosts.contains(&"single.com".to_string()));
                assert!(allowed_email_hosts.contains(&"multi1.com".to_string()));
                assert!(allowed_email_hosts.contains(&"multi2.com".to_string()));
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }

        // Test 13: Verify email_salt is preserved through updates
        let updated_auth = AUTHENTICATORS.load(deps.as_ref().storage, auth_id).unwrap();
        match updated_auth {
            Authenticator::ZKEmail { email_salt, .. } => {
                assert_eq!(email_salt, "test_salt");
            }
            _ => panic!("Expected ZKEmail authenticator"),
        }
    }
}

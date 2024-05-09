use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, Event, Order, Response};

use crate::auth::{passkey, AddAuthenticator, Authenticator};
use crate::{
    error::{ContractError, ContractResult},
    state::AUTHENTICATORS,
};

pub fn init(
    deps: DepsMut,
    env: Env,
    add_authenticator: AddAuthenticator,
) -> ContractResult<Response> {
    add_auth_method(deps, env.clone(), add_authenticator.clone())?;

    Ok(
        Response::new().add_event(Event::new("create_abstract_account").add_attributes(vec![
            ("contract_address", env.contract.address.to_string()),
            (
                "authenticator",
                serde_json::to_string(&add_authenticator).unwrap(),
            ),
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
                // todo: need to validate this with test data
                if sig_bytes.len() < 512 {
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
    env: Env,
    add_authenticator: AddAuthenticator,
) -> ContractResult<Response> {
    match add_authenticator.clone() {
        AddAuthenticator::Secp256K1 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Secp256K1 {
                pubkey: pubkey.clone(),
            };

            if !auth.verify(
                deps.as_ref(),
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, id, &auth)?;
                Ok(())
            }
        }
        AddAuthenticator::Ed25519 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Ed25519 { pubkey };

            if !auth.verify(
                deps.as_ref(),
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, id, &auth)?;
                Ok(())
            }
        }
        AddAuthenticator::EthWallet {
            id,
            address,
            signature,
        } => {
            let auth = Authenticator::EthWallet { address };

            if !auth.verify(
                deps.as_ref(),
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, id, &auth)?;
                Ok(())
            }
        }
        AddAuthenticator::Jwt {
            id,
            aud,
            sub,
            token,
        } => {
            let auth = Authenticator::Jwt { aud, sub };

            if !auth.verify(
                deps.as_ref(),
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &token,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                save_authenticator(deps, id, &auth)?;
                Ok(())
            }
        }
        AddAuthenticator::Secp256R1 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Secp256R1 { pubkey };

            if !auth.verify(
                deps.as_ref(),
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, id, &auth)?;
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
                url.clone(),
                credential,
            )?;

            let auth = Authenticator::Passkey { url, passkey };
            AUTHENTICATORS.save(deps.storage, id, &auth)?;

            Ok(())
        }
    }?;
    Ok(
        Response::new().add_event(Event::new("add_auth_method").add_attributes(vec![
            ("contract_address", env.contract.address.clone().to_string()),
            (
                "authenticator",
                serde_json::to_string(&add_authenticator).unwrap(),
            ),
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
    if AUTHENTICATORS
        .keys(deps.storage, None, None, Order::Ascending)
        .count()
        <= 1
    {
        return Err(ContractError::MinimumAuthenticatorCount);
    }

    AUTHENTICATORS.remove(deps.storage, id);
    Ok(
        Response::new().add_event(Event::new("remove_auth_method").add_attributes(vec![
            ("contract_address", env.contract.address.to_string()),
            ("authenticator_id", id.to_string()),
        ])),
    )
}

pub fn assert_self(sender: &Addr, contract: &Addr) -> ContractResult<()> {
    if sender != contract {
        return Err(ContractError::Unauthorized);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose, Engine as _};
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::Binary;

    use crate::auth::Authenticator;
    use crate::execute::before_tx;
    use crate::state::AUTHENTICATORS;

    #[test]
    fn test_before_tx() {
        let auth_id = 0;
        let mut deps = mock_dependencies();
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
}

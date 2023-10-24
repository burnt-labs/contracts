use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, Order, Response};

use crate::auth::{AddAuthenticator, Authenticator};
use crate::{
    auth,
    error::{ContractError, ContractResult},
    state::AUTHENTICATORS,
};

pub fn init(
    deps: DepsMut,
    env: Env,
    id: u8,
    authenticator: Authenticator,
    signature: &Binary,
) -> ContractResult<Response> {
    if !authenticator.verify(
        deps.api,
        &env,
        &Binary::from(env.contract.address.as_bytes()),
        signature,
    )? {
        return Err(ContractError::InvalidSignature);
    } else {
        AUTHENTICATORS.save(deps.storage, id, &authenticator)?;
    }

    Ok(Response::new()
        .add_attribute("method", "init")
        .add_attribute("authenticator_id", id.to_string()))
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
            Authenticator::Secp256K1 { .. } | auth::Authenticator::Ed25519 { .. } => {
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
        }

        return match authenticator.verify(deps.api, env, tx_bytes, sig_bytes)? {
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
    info: MessageInfo,
    add_authenticator: AddAuthenticator,
) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;
    match add_authenticator {
        AddAuthenticator::Secp256K1 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Secp256K1 { pubkey };

            if !auth.verify(
                deps.api,
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, id, &auth)?;
                Ok(Response::new()
                    .add_attribute("method", "execute")
                    .add_attribute("authenticator_id", id.to_string()))
            }
        }
        AddAuthenticator::Ed25519 {
            id,
            pubkey,
            signature,
        } => {
            let auth = Authenticator::Ed25519 { pubkey };

            if !auth.verify(
                deps.api,
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, id, &auth)?;
                Ok(Response::new()
                    .add_attribute("method", "execute")
                    .add_attribute("authenticator_id", id.to_string()))
            }
        }
        AddAuthenticator::EthWallet {
            id,
            address,
            signature,
        } => {
            let auth = Authenticator::EthWallet { address };

            if !auth.verify(
                deps.api,
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &signature,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, id, &auth)?;
                Ok(Response::new()
                    .add_attribute("method", "execute")
                    .add_attribute("authenticator_id", id.to_string()))
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
                deps.api,
                &env,
                &Binary::from(env.contract.address.as_bytes()),
                &token,
            )? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, id, &auth)?;
                Ok(Response::new()
                    .add_attribute("method", "execute")
                    .add_attribute("authenticator_id", id.to_string()))
            }
        }
    }
}

pub fn remove_auth_method(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: u8,
) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;

    if AUTHENTICATORS
        .keys(deps.storage, None, None, Order::Ascending)
        .count()
        <= 1
    {
        return Err(ContractError::MinimumAuthenticatorCount);
    }

    AUTHENTICATORS.remove(deps.storage, id);
    Ok(Response::new()
        .add_attribute("method", "execute")
        .add_attribute("authenticator_id", id.to_string()))
}

const MAX_SIZE: usize = 1024;
pub fn emit(env: Env, info: MessageInfo, data: String) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;

    if data.len() > MAX_SIZE {
        Err(ContractError::EmissionSizeExceeded)
    } else {
        let emit_event = Event::new("account_emit")
            .add_attribute("contract", env.contract.address)
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
        let authId = 0;
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
        let credIndex = vec![0u8];

        let mut new_vec = Vec::new();
        new_vec.extend_from_slice(&credIndex);
        new_vec.extend_from_slice(&sig_arr);

        AUTHENTICATORS.save(deps.as_mut().storage, authId, &auth);

        let sig_bytes = Binary::from(new_vec);
        let tx_bytes = Binary::from(general_purpose::STANDARD.decode("Cp0BCpoBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEnoKP3hpb24xbTZ2aDIwcHM3NW0ybjZxeHdwandmOGZzM2t4dzc1enN5M3YycnllaGQ5c3BtbnUwcTlyc2g0NnljeRIreGlvbjFlMmZ1d2UzdWhxOHpkOW5ra2s4NzZuYXdyd2R1bGd2NDYwdnpnNxoKCgV1eGlvbhIBMRJTCksKQwodL2Fic3RyYWN0YWNjb3VudC52MS5OaWxQdWJLZXkSIgog3pl1PDD1NqnoBnBk5J0wjYzvUFAkWKGTN2lgHc+PAUcSBAoCCAESBBDgpxIaFHhpb24tbG9jYWwtdGVzdG5ldC0xIAg=").unwrap());

        before_tx(deps.as_ref(), &env, &tx_bytes, Some(&sig_bytes), false).unwrap();
    }
}

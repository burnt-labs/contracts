use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response};

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

pub fn assert_self(sender: &Addr, contract: &Addr) -> ContractResult<()> {
    if sender != contract {
        return Err(ContractError::Unauthorized);
    }

    Ok(())
}

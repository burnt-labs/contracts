use cosmwasm_std::{
    Addr, Binary, CosmosMsg, Deps, DepsMut, Env, IbcMsg, MessageInfo, Order, Response,
};

use crate::auth::{AddAuthenticator, Authenticator};
use crate::{
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
    tx_bytes: &Binary,
    cred_bytes: Option<&Binary>,
    simulate: bool,
) -> ContractResult<Response> {
    if !simulate {
        let cred_bytes = cred_bytes.ok_or(ContractError::EmptySignature)?;
        if cred_bytes.len() < 1 {
            return Err(ContractError::InvalidSignature);
        }

        let cred_index: u8 = match cred_bytes.first() {
            None => return Err(ContractError::InvalidSignature),
            Some(i) => *i,
        };

        let sig_bytes = &Binary::from(&cred_bytes.as_slice()[1..]);

        let auth = AUTHENTICATORS.load(deps.storage, cred_index)?;
        return match auth.verify(deps.api, tx_bytes, sig_bytes)? {
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

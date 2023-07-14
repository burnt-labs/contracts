use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, Storage, Uint64};

use crate::{
    error::{ContractError, ContractResult},
    state::AUTHENTICATORS,
};
use crate::auth::Authenticator;

pub const MAX_AUTHENTICATORS: u8 = 10;

pub fn init(deps: DepsMut, env: Env, id: Uint64, authenticator: Authenticator, signature: &Binary) -> ContractResult<Response> {
    if !authenticator.verify(deps.api, &Binary::from(env.contract.address.as_bytes()), signature)? {
        return Err(ContractError::InvalidSignature);
    } else {
        AUTHENTICATORS.save(deps.storage, u64::to_be_bytes(id.u64()), &authenticator)?;
    }

    Ok(Response::new()
        .add_attribute("method", "init")
        .add_attribute("authenticator_id", id))
}

fn parse_cred_id(cred_id: &[u8]) -> &[u8; 8] {
    cred_id.try_into().expect("incorrect byte length")
}

pub fn before_tx(
    deps: Deps,
    tx_bytes: &Binary,
    cred_bytes: Option<&Binary>,
    simulate: bool,
) -> ContractResult<Response> {
    if !simulate {
        let cred_bytes = cred_bytes.ok_or(ContractError::EmptySignature)?;
        if cred_bytes.len() < 8 {
            return Err(ContractError::InvalidSignature);
        }

        let cred_id: &[u8; 8] = parse_cred_id(&cred_bytes.as_slice()[0..8]);
        let sig_bytes = &Binary::from(&cred_bytes.as_slice()[8..]);

        let auth = AUTHENTICATORS.load(deps.storage, *cred_id)?;
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

pub fn add_auth_method(deps: DepsMut, env: Env, info: MessageInfo, id: Uint64, authenticator: Authenticator, signature: &Binary) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;
    if !authenticator.verify(deps.api, &Binary::from(env.contract.address.as_bytes()), signature)? {
        Err(ContractError::InvalidSignature)
    } else {
        AUTHENTICATORS.save(deps.storage, u64::to_be_bytes(id.u64()), &authenticator)?;
        Ok(Response::new().add_attribute("method", "execute")
            .add_attribute("authenticator_id", id))
    }
}

pub fn remove_auth_method(deps: DepsMut, env: Env, info: MessageInfo, id: Uint64) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;

    if AUTHENTICATORS.keys(deps.storage, None, None, Order::Ascending).count() <= 1 {
        return Err(ContractError::MinimumAuthenticatorCount);
    }

    AUTHENTICATORS.remove(deps.storage, u64::to_be_bytes(id.u64()));
    Ok(Response::new().add_attribute("method", "execute")
        .add_attribute("authenticator_id", id))
}

pub fn assert_self(sender: &Addr, contract: &Addr) -> ContractResult<()> {
    if sender != contract {
        return Err(ContractError::Unauthorized);
    }

    Ok(())
}

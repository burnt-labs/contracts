use cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, Storage, Uint64};
use webauthn_rs::prelude::*;
use webauthn_rs_core::interface::{Credential, RegistrationState};
use webauthn_rs_core::proto::{AuthenticatorAttestationResponseRaw, UserVerificationPolicy};

use crate::{
    error::{ContractError, ContractResult},
    state::AUTHENTICATORS,
};
use crate::auth::{AddAuthenticator, Authenticator};

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

pub fn add_auth_method(deps: DepsMut, env: Env, info: MessageInfo, add_authenticator: AddAuthenticator, signature: &Binary) -> ContractResult<Response> {
    assert_self(&info.sender, &env.contract.address)?;

    match add_authenticator {
        AddAuthenticator::WebAuthN { rp_origin, rp_id, authenticator_attestation } => {
            let rp_origin = match Url::parse(rp_origin.as_str()) {
                Ok(rpo) => rpo,
                Err(error) => return Err(ContractError::Parsing)
            };

            let builder = WebauthnBuilder::new(rp_id.as_str(), &rp_origin)?;
            let webauthn = builder.build()?;

            let rpkc: RegisterPublicKeyCredential = cosmwasm_std::from_slice(authenticator_attestation.as_bytes())?;
            let state = PasskeyRegistration { rs: RegistrationState{
                policy: UserVerificationPolicy::Required,
                exclude_credentials: vec![],
                // the registration challenge is always the contract ID
                // replay attack mitigation is based on users needing another
                // authorization method to submit registrations to the contract,
                // also, as this will be used exclusively to sign tx data,
                // including a nonce, replays should not be a concern
                challenge: Base64UrlSafeData::from(env.contract.address.as_bytes().to_vec()),
                credential_algorithms: vec![],
                require_resident_key: false,
                authenticator_attachment: None,
                extensions: Default::default(),
                experimental_allow_passkeys: false,
            } };
            let passkey = match webauthn.finish_passkey_registration(&rpkc, &state) {
                Ok(p) => p,
                Err(error) => return Err(ContractError::InvalidSignature),
            };
            let cred_id = &passkey.cred_id().clone().0;
            let cred_id_prefix = parse_cred_id(&cred_id[0..8]);
            let passkey_str = cosmwasm_std::to_binary(&passkey)?;
            AUTHENTICATORS.save(deps.storage, *cred_id_prefix,
                                &Authenticator::WebAuthN {
                                    rp_origin: rp_origin.to_string(),
                                    rp_id,
                                    passkey: passkey_str })?;
            Ok(Response::new().add_attribute("method", "execute")
                .add_attribute("authenticator_id", u64::from_be_bytes(*cred_id_prefix).to_string()))
        },
        AddAuthenticator::Secp256K1 { id, pubkey, signature} => {
            let auth = Authenticator::Secp256K1 {pubkey};

            if !auth.verify(deps.api, &Binary::from(env.contract.address.as_bytes()), &signature)? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, u64::to_be_bytes(id.u64()), &auth)?;
                Ok(Response::new().add_attribute("method", "execute")
                    .add_attribute("authenticator_id", id))
            }
        },
        AddAuthenticator::Ed25519 { id, pubkey, signature } => {
            let auth = Authenticator::Ed25519 {pubkey};

            if !auth.verify(deps.api, &Binary::from(env.contract.address.as_bytes()), &signature)? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, u64::to_be_bytes(id.u64()), &auth)?;
                Ok(Response::new().add_attribute("method", "execute")
                    .add_attribute("authenticator_id", id))
            }
        }
        AddAuthenticator::EthWallet { id, address, signature } => {
            let auth = Authenticator::EthWallet {address};

            if !auth.verify(deps.api, &Binary::from(env.contract.address.as_bytes()), &signature)? {
                Err(ContractError::InvalidSignature)
            } else {
                AUTHENTICATORS.save(deps.storage, u64::to_be_bytes(id.u64()), &auth)?;
                Ok(Response::new().add_attribute("method", "execute")
                    .add_attribute("authenticator_id", id))
            }
        }
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

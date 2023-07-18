use std::any::Any;
use crate::eth_crypto;
use cosmwasm_std::{Api, Binary, Deps, Uint64};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Formatter};
use schemars::JsonSchema;
use webauthn_rs::prelude::*;
use webauthn_rs::WebauthnBuilder;
use webauthn_rs_core::interface::{AuthenticationState, Credential};
use webauthn_rs_core::proto::PublicKeyCredential;
use crate::error::ContractError;

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub enum AddAuthenticator {
    Secp256K1 {
        id: Uint64,
        pubkey: Binary,
        signature: Binary,
    },
    Ed25519 {
        id: Uint64,
        pubkey: Binary,
        signature: Binary,
    },
    EthWallet {
        id: Uint64,
        address: String,
        signature: Binary,
    },
    WebAuthN {
        rp_origin: String,
        rp_id: String,
        authenticator_attestation: String,
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub enum Authenticator {
    Secp256K1 { pubkey: Binary },
    Ed25519 { pubkey: Binary },
    EthWallet { address: String },
    // the passkey is serialized to the store
    WebAuthN {
        rp_origin: String,
        rp_id: String,
        passkey: String
    },
}

impl Authenticator {
    pub fn verify(&self, api: &dyn Api, tx_bytes: &Binary, sig_bytes: &Binary) -> Result<bool, ContractError> {
        match self {
            Authenticator::Secp256K1 { pubkey } => {
                let tx_bytes_hash = sha256(tx_bytes);
                match api
                    .secp256k1_verify(&tx_bytes_hash, sig_bytes, pubkey)
                {
                    Ok(verification) => Ok(verification),
                    Err(error) => Err(error.into()),
                }
            }
            Authenticator::Ed25519 { pubkey } => {
                let tx_bytes_hash = sha256(tx_bytes);
                match api
                    .ed25519_verify(&tx_bytes_hash, sig_bytes, pubkey)
                {
                    Ok(verification) => Ok(verification),
                    Err(error) => Err(error.into()),
                }
            }
            Authenticator::EthWallet { address } => {
                let addr_bytes = hex::decode(&address[2..])?;
                match eth_crypto::verify(api, tx_bytes, sig_bytes, &addr_bytes) {
                    Ok(_) => Ok(true),
                    Err(error) => Err(error),
                }
            }
            Authenticator::WebAuthN { rp_origin, rp_id, passkey } => {
                let rp_origin = match Url::parse(rp_origin.as_str()) {
                    Ok(rpo) => rpo,
                    Err(error) => return Err(ContractError::Parsing)
                };

                let builder = WebauthnBuilder::new(rp_id.as_str(), &rp_origin)?;
                let webauthn = builder.build()?;

                let passkey: Passkey = serde_json::from_str(passkey)?;

                let authenticator_response = serde_json::from_slice(sig_bytes.as_slice())?;

                let pkc = PublicKeyCredential{
                    id: passkey.cred_id().to_string(),
                    raw_id: passkey.cred_id().into(),
                    response: authenticator_response,
                    extensions: Default::default(),
                    type_: "public-key".to_string(),
                };
                let passkey_auth = PasskeyAuthentication {
                    ast: AuthenticationState {
                        credentials: vec![Credential::from(passkey)],
                        policy: UserVerificationPolicy::Required,
                        challenge: Base64UrlSafeData::from(tx_bytes),
                        appid: None,
                        allow_backup_eligible_upgrade: false,
                    },
                };
                let result = webauthn.finish_passkey_authentication(&pkc, &passkey_auth)?;
                if !result.user_verified() {
                    Err(ContractError::InvalidSignature)
                }

                Ok(false)
            }
        }
    }
}

pub fn sha256(msg: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(msg);
    hasher.finalize().to_vec()
}

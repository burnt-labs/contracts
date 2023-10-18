use crate::error::ContractError;
use cosmwasm_std::{Api, Binary, Env};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod eth_crypto;
mod jwt;
mod sign_arb;
pub mod util;

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub enum AddAuthenticator {
    Secp256K1 {
        id: u8,
        pubkey: Binary,
        signature: Binary,
    },
    Ed25519 {
        id: u8,
        pubkey: Binary,
        signature: Binary,
    },
    EthWallet {
        id: u8,
        address: String,
        signature: Binary,
    },
    Jwt {
        id: u8,
        aud: String,
        sub: String,
        token: Binary,
    },
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub enum Authenticator {
    Secp256K1 { pubkey: Binary },
    Ed25519 { pubkey: Binary },
    EthWallet { address: String },
    Jwt { aud: String, sub: String },
}

impl Authenticator {
    pub fn verify(
        &self,
        api: &dyn Api,
        env: &Env,
        tx_bytes: &Binary,
        sig_bytes: &Binary,
    ) -> Result<bool, ContractError> {
        match self {
            Authenticator::Secp256K1 { pubkey } => {
                let tx_bytes_hash = util::sha256(tx_bytes);
                let verification = api.secp256k1_verify(&tx_bytes_hash, sig_bytes, pubkey);
                if let Ok(ver) = verification {
                    if ver {
                        return Ok(true);
                    }
                }

                // if the direct verification failed, check to see if they
                // are signing with signArbitrary (common for cosmos wallets)
                let verification = sign_arb::verify(
                    api,
                    tx_bytes.as_slice(),
                    sig_bytes.as_slice(),
                    pubkey.as_slice(),
                )?;
                Ok(verification)
            }
            Authenticator::Ed25519 { pubkey } => {
                let tx_bytes_hash = util::sha256(tx_bytes);
                match api.ed25519_verify(&tx_bytes_hash, sig_bytes, pubkey) {
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
            Authenticator::Jwt { aud, sub } => {
                let tx_bytes_hash = util::sha256(tx_bytes);
                return jwt::verify(
                    &env.block.time,
                    &tx_bytes_hash,
                    sig_bytes.as_slice(),
                    aud,
                    sub,
                );
            }
        }
    }
}

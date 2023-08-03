use crate::eth_crypto;
use cosmwasm_std::{Api, Binary};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use schemars::JsonSchema;
use crate::error::ContractError;

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
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, PartialEq, Debug)]
pub enum Authenticator {
    Secp256K1 { pubkey: Binary },
    Ed25519 { pubkey: Binary },
    EthWallet { address: String },
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
        }
    }
}

pub fn sha256(msg: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(msg);
    hasher.finalize().to_vec()
}

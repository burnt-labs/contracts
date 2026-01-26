//! ZK proof verification module for zkShuffle using XION's module-level zk API.
//!
//! This module provides functions to verify Groth16 proofs for shuffle and decryption operations
//! by querying the XION blockchain's zk module through the Cosmos SDK protobuf interface.

use crate::error::ContractError;
use cosmos_sdk_proto::{
    prost::Message,
    traits::MessageExt,
    xion::v1::zk::{ProofVerifyResponse, QueryVerifyRequest},
};
use cosmwasm_std::{Binary, Deps, QuerierWrapper, Uint256};
use serde::{Deserialize, Serialize};
use serde_json as _;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnarkJsProof {
    #[serde(rename = "pi_a")]
    pub pi_a: [String; 3],
    #[serde(rename = "pi_b")]
    pub pi_b: [[String; 2]; 3],
    #[serde(rename = "pi_c")]
    pub pi_c: [String; 3],
    pub protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve: Option<String>,
}

fn uint256_to_string(val: &Uint256) -> String {
    val.to_string()
}

pub fn groth16_proof_to_snarkjs(
    a: &[Uint256; 2],
    b: &[[Uint256; 2]; 2],
    c: &[Uint256; 2],
) -> SnarkJsProof {
    SnarkJsProof {
        pi_a: [
            uint256_to_string(&a[0]),
            uint256_to_string(&a[1]),
            "1".to_string(), // Groth16 proofs have a third component set to 1
        ],
        pi_b: [
            [uint256_to_string(&b[0][0]), uint256_to_string(&b[0][1])],
            [uint256_to_string(&b[1][0]), uint256_to_string(&b[1][1])],
            ["1".to_string(), "0".to_string()], // Groth16 has a third row [1, 0]
        ],
        pi_c: [
            uint256_to_string(&c[0]),
            uint256_to_string(&c[1]),
            "1".to_string(), // Groth16 proofs have a third component set to 1
        ],
        protocol: "groth16".to_string(),
        curve: Some("bn128".to_string()),
    }
}

/// Convert Uint256 public inputs to string format
pub fn public_inputs_to_string(inputs: &[Uint256]) -> Vec<String> {
    inputs.iter().map(uint256_to_string).collect()
}

/// Verify a shuffle proof using the XION zk module
///
/// # Arguments
/// * `deps` - Deps for querier access
/// * `proof` - Groth16 proof (a, b, c components)
/// * `public_inputs` - Public inputs for the proof circuit
/// * `verifier_name` - Name of the verifier key stored in the zk module
///
/// # Returns
/// * `Ok(true)` if proof is valid
/// * `Ok(false)` if proof is invalid
/// * `Err(ContractError)` if verification fails due to other errors
pub fn verify_shuffle_proof(
    deps: Deps,
    proof: &([Uint256; 2], [[Uint256; 2]; 2], [Uint256; 2]),
    public_inputs: &[Uint256],
    verifier_name: &str,
) -> Result<bool, ContractError> {
    let snarkjs_proof = groth16_proof_to_snarkjs(&proof.0, &proof.1, &proof.2);
    let public_inputs_str = public_inputs_to_string(public_inputs);

    let verify_request = QueryVerifyRequest {
        proof: serde_json::to_vec(&snarkjs_proof).map_err(|e| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "Failed to serialize proof: {}",
                e
            )))
        })?,
        public_inputs: public_inputs_str,
        vkey_name: verifier_name.to_string(),
        vkey_id: 3,
    };

    let request_bytes = verify_request.to_bytes().map_err(|e| {
        ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
            "Failed to encode verify request: {}",
            e
        )))
    })?;

    let response: cosmwasm_std::Binary = deps
        .querier
        .query_grpc(
            "/xion.zk.v1.Query/ProofVerify".to_string(),
            cosmwasm_std::Binary::from(request_bytes),
        )
        .map_err(|e| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "Failed to query zk module: {}",
                e
            )))
        })?;

    let verify_response: ProofVerifyResponse = ProofVerifyResponse::decode(response.as_slice())
        .map_err(|e| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "Failed to decode verify response: {}",
                e
            )))
        })?;

    Ok(verify_response.verified)
}

/// Verify a decryption proof using the XION zk module
///
/// # Arguments
/// * `deps` - Deps for querier access
/// * `proof` - Groth16 proof (a, b, c components)
/// * `public_inputs` - Public inputs for the proof circuit
/// * `verifier_name` - Name of the verifier key stored in the zk module
///
/// # Returns
/// * `Ok(true)` if proof is valid
/// * `Ok(false)` if proof is invalid
/// * `Err(ContractError)` if verification fails due to other errors
pub fn verify_decrypt_proof(
    deps: Deps,
    proof: &([Uint256; 2], [[Uint256; 2]; 2], [Uint256; 2]),
    public_inputs: &[Uint256],
    verifier_name: &str,
) -> Result<bool, ContractError> {
    let snarkjs_proof = groth16_proof_to_snarkjs(&proof.0, &proof.1, &proof.2);
    let public_inputs_str = public_inputs_to_string(public_inputs);

    let verify_request = QueryVerifyRequest {
        proof: serde_json::to_vec(&snarkjs_proof).map_err(|e| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "Failed to serialize proof: {}",
                e
            )))
        })?,
        public_inputs: public_inputs_str,
        vkey_name: verifier_name.to_string(),
        vkey_id: 2,
    };

    let request_bytes = verify_request.to_bytes().map_err(|e| {
        ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
            "Failed to encode verify request: {}",
            e
        )))
    })?;

    let response: cosmwasm_std::Binary = deps
        .querier
        .query_grpc(
            "/xion.zk.v1.Query/ProofVerify".to_string(),
            cosmwasm_std::Binary::from(request_bytes),
        )
        .map_err(|e| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "Failed to query zk module: {}",
                e
            )))
        })?;

    let verify_response: ProofVerifyResponse = ProofVerifyResponse::decode(response.as_slice())
        .map_err(|e| {
            ContractError::Std(cosmwasm_std::StdError::generic_err(format!(
                "Failed to decode verify response: {}",
                e
            )))
        })?;

    Ok(verify_response.verified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groth16_proof_conversion() {
        use std::str::FromStr;

        let a = [
            Uint256::from_str("123456").unwrap(),
            Uint256::from_str("789012").unwrap(),
        ];
        let b = [
            [
                Uint256::from_str("345678").unwrap(),
                Uint256::from_str("901234").unwrap(),
            ],
            [
                Uint256::from_str("567890").unwrap(),
                Uint256::from_str("123456").unwrap(),
            ],
        ];
        let c = [
            Uint256::from_str("234567").unwrap(),
            Uint256::from_str("890123").unwrap(),
        ];

        let snarkjs = groth16_proof_to_snarkjs(&a, &b, &c);

        assert_eq!(snarkjs.pi_a[0], "123456");
        assert_eq!(snarkjs.pi_a[1], "789012");
        assert_eq!(snarkjs.pi_a[2], "1");
        assert_eq!(snarkjs.pi_b[0][0], "345678");
        assert_eq!(snarkjs.pi_b[0][1], "901234");
        assert_eq!(snarkjs.pi_b[1][0], "567890");
        assert_eq!(snarkjs.pi_b[1][1], "123456");
        assert_eq!(snarkjs.pi_b[2][0], "1");
        assert_eq!(snarkjs.pi_b[2][1], "0");
        assert_eq!(snarkjs.pi_c[0], "234567");
        assert_eq!(snarkjs.pi_c[1], "890123");
        assert_eq!(snarkjs.pi_c[2], "1");
        assert_eq!(snarkjs.protocol, "groth16");
        assert_eq!(snarkjs.curve, Some("bn128".to_string()));
    }

    #[test]
    fn test_snarkjs_proof_serialization() {
        let proof = SnarkJsProof {
            pi_a: ["1".to_string(), "2".to_string(), "3".to_string()],
            pi_b: [
                ["4".to_string(), "5".to_string()],
                ["6".to_string(), "7".to_string()],
                ["8".to_string(), "9".to_string()],
            ],
            pi_c: ["10".to_string(), "11".to_string(), "12".to_string()],
            protocol: "groth16".to_string(),
            curve: Some("bn128".to_string()),
        };

        let serialized = serde_json::to_string(&proof).unwrap();
        let deserialized: SnarkJsProof = serde_json::from_str(&serialized).unwrap();

        assert_eq!(proof.pi_a, deserialized.pi_a);
        assert_eq!(proof.pi_b, deserialized.pi_b);
        assert_eq!(proof.pi_c, deserialized.pi_c);
        assert_eq!(proof.protocol, deserialized.protocol);
        assert_eq!(proof.curve, deserialized.curve);
    }

    #[test]
    fn test_public_inputs_to_string() {
        use std::str::FromStr;

        let inputs = vec![
            Uint256::from_str("123").unwrap(),
            Uint256::from_str("456").unwrap(),
            Uint256::from_str("789").unwrap(),
        ];

        let strings = public_inputs_to_string(&inputs);

        assert_eq!(strings, vec!["123", "456", "789"]);
    }
}

//! Minimal zkShuffle contract for testing proof verification at XION module level

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, VerificationCountResponse};
use crate::state::{VerificationState, VERIFICATION_STATE};
use crate::types::Groth16Proof;
use crate::zkshuffle::{verify_decrypt_proof, verify_shuffle_proof};

const CONTRACT_NAME: &str = "crates.io:zk-shuffle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = VerificationState::new();
    VERIFICATION_STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::VerifyShuffleProof {
            proof,
            public_inputs,
        } => execute_verify_shuffle_proof(deps, proof, public_inputs),
        ExecuteMsg::VerifyDecryptProof {
            proof,
            public_inputs,
        } => execute_verify_decrypt_proof(deps, proof, public_inputs),
    }
}

fn execute_verify_shuffle_proof(
    deps: DepsMut,
    proof: Groth16Proof,
    public_inputs: Vec<cosmwasm_std::Uint256>,
) -> Result<Response, ContractError> {
    let proof_tuple = (proof.a, proof.b, proof.c);
    let verifier_name = "shuffle_encrypt";
    let verified =
        verify_shuffle_proof(deps.as_ref(), &proof_tuple, &public_inputs, verifier_name)?;

    if !verified {
        return Err(ContractError::InvalidProof);
    }
    let mut state = VERIFICATION_STATE.load(deps.storage)?;
    state.shuffle_verifications += 1;
    VERIFICATION_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "verify_shuffle_proof")
        .add_attribute("result", "success")
        .add_attribute(
            "total_shuffle_verifications",
            state.shuffle_verifications.to_string(),
        ))
}

fn execute_verify_decrypt_proof(
    deps: DepsMut,
    proof: Groth16Proof,
    public_inputs: Vec<cosmwasm_std::Uint256>,
) -> Result<Response, ContractError> {
    let proof_tuple = (proof.a, proof.b, proof.c);
    let verifier_name = "decrypt";
    let verified =
        verify_decrypt_proof(deps.as_ref(), &proof_tuple, &public_inputs, verifier_name)?;

    if !verified {
        return Err(ContractError::InvalidProof);
    }
    let mut state = VERIFICATION_STATE.load(deps.storage)?;
    state.decrypt_verifications += 1;
    VERIFICATION_STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "verify_decrypt_proof")
        .add_attribute("result", "success")
        .add_attribute(
            "total_decrypt_verifications",
            state.decrypt_verifications.to_string(),
        ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VerificationCount {} => to_json_binary(&query_verification_count(deps)?),
    }
}

fn query_verification_count(deps: Deps) -> StdResult<VerificationCountResponse> {
    let state = VERIFICATION_STATE.load(deps.storage)?;
    Ok(VerificationCountResponse {
        shuffle_verifications: state.shuffle_verifications,
        decrypt_verifications: state.decrypt_verifications,
    })
}

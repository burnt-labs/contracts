use core::fmt;

use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env as CwEnv, MessageInfo, Response,
    StdError,
};
use cw_storage_plus::Item;

use crate::error::RejectCode;
use crate::machine::{DossierState, Env};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::Config;

pub const STATE: Item<DossierState> = Item::new("dossier_state");

#[derive(Debug)]
pub enum ContractError {
    Std(StdError),
    Reject(RejectCode),
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Std(e) => write!(f, "{e}"),
            Self::Reject(c) => f.write_str(c.as_str()),
        }
    }
}

impl std::error::Error for ContractError {}

impl From<StdError> for ContractError {
    fn from(e: StdError) -> Self {
        Self::Std(e)
    }
}

impl From<RejectCode> for ContractError {
    fn from(c: RejectCode) -> Self {
        Self::Reject(c)
    }
}

#[cfg(feature = "mock-attestation")]
fn verifier(
    _querier: cosmwasm_std::QuerierWrapper,
    _env: &CwEnv,
    _state: &DossierState,
) -> crate::machine::mock::MockVerifier {
    crate::machine::mock::MockVerifier { epoch: 1 }
}

#[cfg(all(not(feature = "mock-attestation"), feature = "accepting-proof"))]
fn verifier(
    _querier: cosmwasm_std::QuerierWrapper,
    env: &CwEnv,
    state: &DossierState,
) -> crate::quote::QuoteVerifier<crate::quote::AcceptingBackend> {
    crate::quote::QuoteVerifier {
        backend: crate::quote::AcceptingBackend,
        chain_id: env.block.chain_id.clone(),
        now_packed: crate::quote::unix_to_packed_datetime(env.block.time.seconds()),
        min_tcb_eval_num: state.config.min_tcb_eval_num,
    }
}

#[cfg(all(not(feature = "mock-attestation"), not(feature = "accepting-proof")))]
fn verifier<'a>(
    querier: cosmwasm_std::QuerierWrapper<'a>,
    env: &CwEnv,
    state: &DossierState,
) -> crate::quote::QuoteVerifier<crate::quote::xion::XionUltraHonkBackend<'a>> {
    crate::quote::QuoteVerifier {
        backend: crate::quote::xion::XionUltraHonkBackend::by_name(
            querier,
            state.config.vkey_name.clone(),
        ),
        chain_id: env.block.chain_id.clone(),
        now_packed: crate::quote::unix_to_packed_datetime(env.block.time.seconds()),
        min_tcb_eval_num: state.config.min_tcb_eval_num,
    }
}

fn machine_env(env: &CwEnv, info: &MessageInfo) -> Env {
    Env {
        now: env.block.time.seconds(),
        sender: info.sender.to_string(),
        chain_id: env.block.chain_id.clone(),
    }
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: CwEnv,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        snapshot_retention_floor_seconds: msg.snapshot_retention_floor_seconds,
        schema_registry: msg.schema_registry.into_iter().collect(),
        vkey_name: msg.vkey_name,
        min_tcb_eval_num: msg.min_tcb_eval_num,
    };
    if !config.validate() {
        return Err(RejectCode::InvalidConfig.into());
    }
    let state = DossierState::new(
        info.sender.to_string(),
        env.contract.address.to_string(),
        config,
    );
    STATE.save(deps.storage, &state)?;
    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: CwEnv,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage)?;
    let menv = machine_env(&env, &info);
    let querier = deps.querier;
    let v = verifier(querier, &env, &state);

    let resp = match msg {
        ExecuteMsg::Admit {
            schema_id,
            ciphertext,
            proof_blob,
            provenance,
        } => {
            let id = state.admit(&menv, schema_id, ciphertext, proof_blob, provenance)?;
            Response::new()
                .add_attribute("action", "admit")
                .add_attribute("admission_id", id.to_string())
        }
        ExecuteMsg::AdmissionAccept {
            admission_id,
            attestation,
        } => {
            state.admission_accept(&v, &menv, admission_id, attestation)?;
            Response::new()
                .add_attribute("action", "admission_accept")
                .add_attribute("entry_id", admission_id.to_string())
        }
        ExecuteMsg::AdmissionReject {
            admission_id,
            reason,
            attestation,
        } => {
            state.admission_reject(&v, &menv, admission_id, reason, attestation)?;
            Response::new()
                .add_attribute("action", "admission_reject")
                .add_attribute("admission_id", admission_id.to_string())
        }
        ExecuteMsg::CancelPendingAdmission { admission_id } => {
            state.cancel_pending_admission(&menv, admission_id)?;
            Response::new()
                .add_attribute("action", "cancel_pending_admission")
                .add_attribute("admission_id", admission_id.to_string())
        }
        ExecuteMsg::Revoke { entry_id } => {
            state.revoke(&menv, entry_id)?;
            Response::new()
                .add_attribute("action", "revoke")
                .add_attribute("entry_id", entry_id.to_string())
        }
        ExecuteMsg::CreateDisclosure {
            encrypted_request_blob,
        } => {
            let id = state.create_disclosure(&menv, encrypted_request_blob)?;
            Response::new()
                .add_attribute("action", "create_disclosure")
                .add_attribute("disclosure_id", id.to_string())
        }
        ExecuteMsg::FulfilDisclosure {
            disclosure_id,
            disclosure_ciphertext,
            snapshot_root,
            pk_c_hash,
            attestation,
        } => {
            state.fulfil_disclosure(
                &v,
                &menv,
                disclosure_id,
                disclosure_ciphertext,
                snapshot_root,
                pk_c_hash,
                attestation,
            )?;
            Response::new()
                .add_attribute("action", "fulfil_disclosure")
                .add_attribute("disclosure_id", disclosure_id.to_string())
        }
        ExecuteMsg::FailDisclosure {
            disclosure_id,
            reason,
            fail_reason_ciphertext,
            snapshot_root,
            pk_slot,
            attestation,
        } => {
            state.fail_disclosure(
                &v,
                &menv,
                disclosure_id,
                reason,
                fail_reason_ciphertext,
                snapshot_root,
                pk_slot,
                attestation,
            )?;
            Response::new()
                .add_attribute("action", "fail_disclosure")
                .add_attribute("disclosure_id", disclosure_id.to_string())
                .add_attribute("reason", format!("{reason:?}"))
        }
        ExecuteMsg::CancelPendingDisclosure { disclosure_id } => {
            state.cancel_pending_disclosure(&menv, disclosure_id)?;
            Response::new()
                .add_attribute("action", "cancel_pending_disclosure")
                .add_attribute("disclosure_id", disclosure_id.to_string())
        }
        ExecuteMsg::PruneDisclosure { disclosure_id } => {
            state.prune_disclosure(&menv, disclosure_id)?;
            Response::new()
                .add_attribute("action", "prune_disclosure")
                .add_attribute("disclosure_id", disclosure_id.to_string())
        }
        ExecuteMsg::RegisterEnclaveKey {
            pubkey,
            attestation,
        } => {
            state.register_enclave_key(&v, &menv, pubkey, attestation)?;
            Response::new().add_attribute("action", "register_enclave_key")
        }
        ExecuteMsg::ProposeKeyRotation {
            new_pubkey,
            attestation,
        } => {
            state.propose_key_rotation(&v, &menv, new_pubkey, attestation)?;
            Response::new().add_attribute("action", "propose_key_rotation")
        }
        ExecuteMsg::FinalizeKeyRotation {} => {
            state.finalize_key_rotation(&v, &menv)?;
            Response::new().add_attribute("action", "finalize_key_rotation")
        }
        ExecuteMsg::CancelKeyRotation {} => {
            state.cancel_key_rotation(&menv)?;
            Response::new().add_attribute("action", "cancel_key_rotation")
        }
    };

    STATE.save(deps.storage, &state)?;
    Ok(resp)
}

#[entry_point]
pub fn query(deps: Deps, _env: CwEnv, msg: QueryMsg) -> Result<Binary, ContractError> {
    let state = STATE.load(deps.storage)?;
    let bin = match msg {
        QueryMsg::State {} => to_json_binary(&state)?,
        QueryMsg::EntriesRoot {} => to_json_binary(&state.entries_root)?,
        QueryMsg::EnclavePubkey {} => to_json_binary(&state.enclave_pubkey)?,
    };
    Ok(bin)
}

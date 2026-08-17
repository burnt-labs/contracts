use crate::error::RejectCode;
use crate::merkle::{self, sha256};
use crate::msg::AdmissionRejectReason;
use crate::state::{
    dossier_derivation_path, sentinel_pk_unrecoverable, sentinel_snapshot_unavailable, AdmissionId,
    Config, DisclosureId, DisclosureOutcome, Entry, EntryId, FailReason, FinalizedDisclosure,
    Hash32, Id, PendingAdmission, PendingDisclosure, PendingEnclaveKey, Provenance,
    KEY_ROTATION_TIMELOCK_SECONDS, RETENTION_WINDOW_SECONDS,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Env {
    pub now: u64,
    pub sender: String,
    pub chain_id: String,
}

pub use dossier_protocol::binding::*;

fn envelope_well_formed(bytes: &[u8]) -> bool {
    crate::envelope::well_formed(bytes)
}

fn proof_blob_well_formed(bytes: &[u8]) -> bool {
    !bytes.is_empty()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DossierState {
    pub owner: String,
    pub contract_address: String,
    pub config: Config,
    pub next_id: Id,
    pub enclave_pubkey: Option<Vec<u8>>,
    pub pending_enclave_key: Option<PendingEnclaveKey>,
    pub previous_enclave_pubkey_hash: Option<Hash32>,
    pub entries: BTreeMap<EntryId, Entry>,
    pub entries_root: Hash32,
    pub pending_admissions: BTreeMap<AdmissionId, PendingAdmission>,
    pub pending_disclosures: BTreeMap<DisclosureId, PendingDisclosure>,
    pub disclosures: BTreeMap<DisclosureId, FinalizedDisclosure>,
}

impl DossierState {
    pub fn new(owner: String, contract_address: String, config: Config) -> Self {
        Self {
            owner,
            contract_address,
            config,
            next_id: 1,
            enclave_pubkey: None,
            pending_enclave_key: None,
            previous_enclave_pubkey_hash: None,
            entries: BTreeMap::new(),
            entries_root: merkle::empty_entries_root(),
            pending_admissions: BTreeMap::new(),
            pending_disclosures: BTreeMap::new(),
            disclosures: BTreeMap::new(),
        }
    }

    fn alloc_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("next_id overflow");
        id
    }

    fn recompute_root(&mut self) {
        self.entries_root = merkle::entries_root_of_map(&self.entries);
    }

    fn require_owner(&self, env: &Env) -> Result<(), RejectCode> {
        if env.sender == self.owner {
            Ok(())
        } else {
            Err(RejectCode::Unauthorized)
        }
    }

    fn current_key_hash(&self) -> Option<Hash32> {
        self.enclave_pubkey.as_deref().map(sha256)
    }

    fn verify_enclave_binding(
        &self,
        verifier: &impl AttestationVerifier,
        attestation: &[u8],
        binding_with_current_key: &Binding,
    ) -> Result<(), RejectCode> {
        if verifier.verify_binding(attestation, binding_with_current_key) {
            return Ok(());
        }
        if let Some(prev) = self.previous_enclave_pubkey_hash {
            let probe = binding_with_current_key.with_enclave_pubkey_hash(prev);
            if verifier.verify_binding(attestation, &probe) {
                return Err(RejectCode::KeyMismatch);
            }
        }
        Err(RejectCode::InvalidAttestation)
    }
    pub fn admit(
        &mut self,
        env: &Env,
        schema_id: String,
        ciphertext: Vec<u8>,
        proof_blob: Vec<u8>,
        provenance: Provenance,
    ) -> Result<AdmissionId, RejectCode> {
        if self.enclave_pubkey.is_none() {
            return Err(RejectCode::NoKeySet);
        }
        if self.pending_enclave_key.is_some() {
            return Err(RejectCode::PendingRotation);
        }
        if !self.config.schema_registry.contains(&schema_id) {
            return Err(RejectCode::UnsupportedSchema);
        }
        if !proof_blob_well_formed(&proof_blob) {
            return Err(RejectCode::InvalidProofFormat);
        }
        if !envelope_well_formed(&ciphertext) {
            return Err(RejectCode::InvalidEnvelope);
        }
        let id = self.alloc_id();
        self.pending_admissions.insert(
            id,
            PendingAdmission {
                schema_id,
                ciphertext,
                proof_blob,
                provenance,
                submitted_ts: env.now,
            },
        );
        Ok(id)
    }

    pub fn admission_accept(
        &mut self,
        verifier: &impl AttestationVerifier,
        env: &Env,
        admission_id: AdmissionId,
        attestation: Vec<u8>,
    ) -> Result<(), RejectCode> {
        let pa = self
            .pending_admissions
            .get(&admission_id)
            .ok_or(RejectCode::NotFound)?;
        let pk_hash = self.current_key_hash().ok_or(RejectCode::NoKeySet)?;
        let binding = Binding::AdmissionAccept {
            chain_id: &env.chain_id,
            contract_address: &self.contract_address,
            admission_id,
            ciphertext_hash: sha256(&pa.ciphertext),
            schema_id: &pa.schema_id,
            enclave_pubkey_hash: pk_hash,
        };
        self.verify_enclave_binding(verifier, &attestation, &binding)?;
        let pa = self
            .pending_admissions
            .remove(&admission_id)
            .expect("checked");
        self.entries.insert(
            admission_id,
            Entry {
                schema_id: pa.schema_id,
                ciphertext: pa.ciphertext,
                provenance: pa.provenance,
                admission_ts: pa.submitted_ts,
                admission_attestation: attestation,
                enclave_pubkey_hash_at_admission: pk_hash,
            },
        );
        self.recompute_root();
        Ok(())
    }

    pub fn admission_reject(
        &mut self,
        verifier: &impl AttestationVerifier,
        env: &Env,
        admission_id: AdmissionId,
        reason: AdmissionRejectReason,
        attestation: Vec<u8>,
    ) -> Result<(), RejectCode> {
        let pa = self
            .pending_admissions
            .get(&admission_id)
            .ok_or(RejectCode::NotFound)?;
        let pk_hash = self.current_key_hash().ok_or(RejectCode::NoKeySet)?;
        let binding = Binding::AdmissionReject {
            chain_id: &env.chain_id,
            contract_address: &self.contract_address,
            admission_id,
            reason,
            ciphertext_hash: sha256(&pa.ciphertext),
            schema_id: &pa.schema_id,
            enclave_pubkey_hash: pk_hash,
        };
        self.verify_enclave_binding(verifier, &attestation, &binding)?;
        self.pending_admissions.remove(&admission_id);
        Ok(())
    }

    pub fn cancel_pending_admission(
        &mut self,
        env: &Env,
        admission_id: AdmissionId,
    ) -> Result<(), RejectCode> {
        self.require_owner(env)?;
        self.pending_admissions
            .remove(&admission_id)
            .map(|_| ())
            .ok_or(RejectCode::NotFound)
    }

    pub fn revoke(&mut self, env: &Env, entry_id: EntryId) -> Result<(), RejectCode> {
        self.require_owner(env)?;
        self.entries.remove(&entry_id).ok_or(RejectCode::NotFound)?;
        self.recompute_root();
        Ok(())
    }

    pub fn create_disclosure(
        &mut self,
        env: &Env,
        encrypted_request_blob: Vec<u8>,
    ) -> Result<DisclosureId, RejectCode> {
        self.require_owner(env)?;
        if self.enclave_pubkey.is_none() {
            return Err(RejectCode::NoKeySet);
        }
        if self.pending_enclave_key.is_some() {
            return Err(RejectCode::PendingRotation);
        }
        if !envelope_well_formed(&encrypted_request_blob) {
            return Err(RejectCode::InvalidEnvelope);
        }
        let id = self.alloc_id();
        self.pending_disclosures.insert(
            id,
            PendingDisclosure {
                encrypted_request_blob,
                created_ts: env.now,
                entries_root_at_create: self.entries_root,
            },
        );
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fulfil_disclosure(
        &mut self,
        verifier: &impl AttestationVerifier,
        env: &Env,
        disclosure_id: DisclosureId,
        disclosure_ciphertext: Vec<u8>,
        snapshot_root: Hash32,
        pk_c_hash: Hash32,
        attestation: Vec<u8>,
    ) -> Result<(), RejectCode> {
        let pd = self
            .pending_disclosures
            .get(&disclosure_id)
            .ok_or(RejectCode::NotFound)?;
        let pk_hash = self.current_key_hash().ok_or(RejectCode::NoKeySet)?;
        if snapshot_root != pd.entries_root_at_create {
            return Err(RejectCode::SnapshotMismatch);
        }
        let binding = Binding::Fulfil {
            chain_id: &env.chain_id,
            contract_address: &self.contract_address,
            disclosure_id,
            request_blob_hash: sha256(&pd.encrypted_request_blob),
            disclosure_ciphertext_hash: sha256(&disclosure_ciphertext),
            snapshot_root,
            pk_c_hash,
            enclave_pubkey_hash: pk_hash,
        };
        self.verify_enclave_binding(verifier, &attestation, &binding)?;
        let pd = self
            .pending_disclosures
            .remove(&disclosure_id)
            .expect("checked");
        self.disclosures.insert(
            disclosure_id,
            FinalizedDisclosure {
                outcome: DisclosureOutcome::Fulfilled {
                    disclosure_ciphertext,
                },
                attestation,
                snapshot_root,
                created_ts: pd.created_ts,
                finalized_ts: env.now,
            },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fail_disclosure(
        &mut self,
        verifier: &impl AttestationVerifier,
        env: &Env,
        disclosure_id: DisclosureId,
        reason: FailReason,
        fail_reason_ciphertext: Vec<u8>,
        snapshot_root: Hash32,
        pk_slot: Hash32,
        attestation: Vec<u8>,
    ) -> Result<(), RejectCode> {
        let pd = self
            .pending_disclosures
            .get(&disclosure_id)
            .ok_or(RejectCode::NotFound)?;
        let pk_hash = self.current_key_hash().ok_or(RejectCode::NoKeySet)?;

        if reason == FailReason::SnapshotUnavailable {
            if snapshot_root != sentinel_snapshot_unavailable() {
                return Err(RejectCode::SnapshotMismatch);
            }
            if pd.entries_root_at_create == merkle::empty_entries_root() {
                return Err(RejectCode::SnapshotMismatch);
            }
            let old_enough = env
                .now
                .checked_sub(pd.created_ts)
                .is_some_and(|age| age >= self.config.snapshot_retention_floor_seconds);
            if !old_enough {
                return Err(RejectCode::SnapshotNotOldEnough);
            }
        } else if snapshot_root != pd.entries_root_at_create {
            return Err(RejectCode::SnapshotMismatch);
        }

        let slot_is_sentinel = pk_slot == sentinel_pk_unrecoverable();
        let reason_is_rdf = reason == FailReason::RequestDecryptionFailed;
        if slot_is_sentinel != reason_is_rdf {
            return Err(RejectCode::InvalidAttestation);
        }
        if reason_is_rdf && !fail_reason_ciphertext.is_empty() {
            return Err(RejectCode::InvalidAttestation);
        }

        let binding = Binding::Fail {
            chain_id: &env.chain_id,
            contract_address: &self.contract_address,
            disclosure_id,
            reason,
            request_blob_hash: sha256(&pd.encrypted_request_blob),
            fail_reason_ciphertext_hash: sha256(&fail_reason_ciphertext),
            snapshot_root,
            pk_slot,
            enclave_pubkey_hash: pk_hash,
        };
        self.verify_enclave_binding(verifier, &attestation, &binding)?;
        let pd = self
            .pending_disclosures
            .remove(&disclosure_id)
            .expect("checked");
        self.disclosures.insert(
            disclosure_id,
            FinalizedDisclosure {
                outcome: DisclosureOutcome::Failed {
                    reason,
                    fail_reason_ciphertext,
                },
                attestation,
                snapshot_root,
                created_ts: pd.created_ts,
                finalized_ts: env.now,
            },
        );
        Ok(())
    }

    pub fn cancel_pending_disclosure(
        &mut self,
        env: &Env,
        disclosure_id: DisclosureId,
    ) -> Result<(), RejectCode> {
        self.require_owner(env)?;
        self.pending_disclosures
            .remove(&disclosure_id)
            .map(|_| ())
            .ok_or(RejectCode::NotFound)
    }

    pub fn prune_disclosure(
        &mut self,
        env: &Env,
        disclosure_id: DisclosureId,
    ) -> Result<(), RejectCode> {
        let fd = self
            .disclosures
            .get(&disclosure_id)
            .ok_or(RejectCode::NotFound)?;
        let elapsed = env
            .now
            .checked_sub(fd.finalized_ts)
            .is_some_and(|e| e >= RETENTION_WINDOW_SECONDS);
        if !elapsed {
            return Err(RejectCode::RetentionWindowNotElapsed);
        }
        self.disclosures.remove(&disclosure_id);
        Ok(())
    }

    pub fn register_enclave_key(
        &mut self,
        verifier: &impl AttestationVerifier,
        env: &Env,
        pubkey: Vec<u8>,
        attestation: Vec<u8>,
    ) -> Result<(), RejectCode> {
        self.require_owner(env)?;
        if self.enclave_pubkey.is_some() {
            return Err(RejectCode::KeyAlreadySet);
        }
        if pubkey.len() != crate::envelope::COMPRESSED_PUBKEY_LEN {
            return Err(RejectCode::MalformedEnclaveKey);
        }
        let path = dossier_derivation_path(&self.contract_address);
        let binding = KeyBinding {
            pubkey: &pubkey,
            contract_address: &self.contract_address,
            derivation_path: &path,
            purpose: KeyPurpose::Register,
        };
        match verifier.verify_key_binding(&attestation, &binding) {
            Ok(_) => {}
            Err(KeyBindingError::Invalid) => return Err(RejectCode::InvalidAttestation),
            Err(KeyBindingError::BindingMismatch) => {
                return Err(RejectCode::DossierBindingMismatch)
            }
        };
        self.enclave_pubkey = Some(pubkey);
        Ok(())
    }
    pub fn propose_key_rotation(
        &mut self,
        verifier: &impl AttestationVerifier,
        env: &Env,
        new_pubkey: Vec<u8>,
        attestation: Vec<u8>,
    ) -> Result<(), RejectCode> {
        self.require_owner(env)?;
        let Some(current) = self.enclave_pubkey.as_deref() else {
            return Err(RejectCode::NoKeySet);
        };
        if self.pending_enclave_key.is_some() {
            return Err(RejectCode::PendingRotation);
        }
        if new_pubkey == current {
            return Err(RejectCode::IdenticalKey);
        }
        if new_pubkey.len() != crate::envelope::COMPRESSED_PUBKEY_LEN {
            return Err(RejectCode::MalformedEnclaveKey);
        }
        let path = dossier_derivation_path(&self.contract_address);
        let binding = KeyBinding {
            pubkey: &new_pubkey,
            contract_address: &self.contract_address,
            derivation_path: &path,
            purpose: KeyPurpose::Rotate,
        };

        match verifier.verify_key_binding(&attestation, &binding) {
            Ok(_) => {}
            Err(KeyBindingError::Invalid) => return Err(RejectCode::InvalidAttestation),
            Err(KeyBindingError::BindingMismatch) => {
                return Err(RejectCode::DossierBindingMismatch)
            }
        };
        self.pending_enclave_key = Some(PendingEnclaveKey {
            key: new_pubkey,
            attestation,
            proposed_at: env.now,
            unlock_time: env.now.saturating_add(KEY_ROTATION_TIMELOCK_SECONDS),
        });
        Ok(())
    }

    pub fn finalize_key_rotation(
        &mut self,
        verifier: &impl AttestationVerifier,
        env: &Env,
    ) -> Result<(), RejectCode> {
        let Some(pending) = self.pending_enclave_key.as_ref() else {
            return Err(RejectCode::NoRotationPending);
        };
        if env.now < pending.unlock_time {
            return Err(RejectCode::TimelockNotElapsed);
        }
        let path = dossier_derivation_path(&self.contract_address);
        let binding = KeyBinding {
            pubkey: &pending.key,
            contract_address: &self.contract_address,
            derivation_path: &path,
            purpose: KeyPurpose::Rotate,
        };
        if verifier
            .verify_key_binding(&pending.attestation, &binding)
            .is_err()
        {
            return Err(RejectCode::AttestationExpired);
        }
        let pending = self.pending_enclave_key.take().expect("checked");
        self.previous_enclave_pubkey_hash = self.current_key_hash();
        self.enclave_pubkey = Some(pending.key);
        Ok(())
    }
    pub fn cancel_key_rotation(&mut self, env: &Env) -> Result<(), RejectCode> {
        self.require_owner(env)?;
        if self.pending_enclave_key.is_none() {
            return Err(RejectCode::NoRotationPending);
        }
        self.pending_enclave_key = None;
        Ok(())
    }
}

#[cfg(any(test, feature = "mock-attestation"))]
pub use dossier_protocol::mock;

#[cfg(test)]
mod tests;

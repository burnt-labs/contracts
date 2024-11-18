use crate::auth::groth16::{GrothBn, GrothBnProof, GrothBnVkey, GrothFp};
use crate::error::ContractError::InvalidDkim;
use crate::error::ContractResult;
use ark_crypto_primitives::snark::SNARK;
use ark_ff::{PrimeField, Zero};
use ark_serialize::CanonicalDeserialize;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use cosmos_sdk_proto::prost::Message;
use cosmos_sdk_proto::traits::MessageExt;
use cosmos_sdk_proto::xion::v1::dkim::{QueryDkimPubKeysRequest, QueryDkimPubKeysResponse};
use cosmwasm_std::{Binary, Deps};

const TX_BODY_MAX_BYTES: usize = 512;

pub fn calculate_tx_body_commitment(tx: &str) -> GrothFp {
    let padded_tx_bytes = pad_bytes(tx.as_bytes(), TX_BODY_MAX_BYTES);
    let tx = pack_bytes_into_fields(padded_tx_bytes);
    let poseidon = poseidon_ark::Poseidon::new();
    let mut commitment = GrothFp::zero(); // Initialize commitment with an initial value

    tx.chunks(16).enumerate().for_each(|(i, chunk)| {
        let chunk_commitment = poseidon.hash(chunk.to_vec()).unwrap();
        commitment = if i == 0 {
            chunk_commitment
        } else {
            poseidon.hash(vec![commitment, chunk_commitment]).unwrap()
        };
    });

    commitment
}

fn pack_bytes_into_fields(bytes: Vec<u8>) -> Vec<GrothFp> {
    // convert each 31 bytes into one field element
    let mut fields = vec![];
    bytes.chunks(31).for_each(|chunk| {
        fields.push(GrothFp::from_le_bytes_mod_order(&chunk));
    });
    fields
}

fn pad_bytes(bytes: &[u8], length: usize) -> Vec<u8> {
    let mut padded = bytes.to_vec();
    let padding = length - bytes.len();
    for _ in 0..padding {
        padded.push(0);
    }
    padded
}

pub fn verify(
    deps: Deps,
    tx_bytes: &Binary,
    sig_bytes: &Binary,
    vkey_bytes: &Binary,
    email_hash: &Binary,
    dkim_domain: &String,
) -> ContractResult<bool> {
    // vkey serialization is checked on submission
    let vkey = GrothBnVkey::deserialize_compressed_unchecked(vkey_bytes.as_slice())?;

    let (dkim_hash_bz, proof_bz) = sig_bytes.split_at(256);

    // proof submission is from the tx, we can't be sure if it was properly serialized
    let proof = GrothBnProof::deserialize_compressed(proof_bz)?;

    // inputs are tx body, email hash, and dmarc key hash
    let mut inputs: [GrothFp; 3] = [GrothFp::zero(); 3];

    // tx body input
    let tx_input = calculate_tx_body_commitment(STANDARD_NO_PAD.encode(tx_bytes).as_str());
    inputs[0] = tx_input;

    // email hash input, compressed at authenticator registration
    let email_hash_input = GrothFp::deserialize_compressed_unchecked(email_hash.as_slice())?;
    inputs[1] = email_hash_input;

    // verify that domain+hash are known in chain state
    let query = QueryDkimPubKeysRequest {
        selector: "".to_string(),
        domain: dkim_domain.to_string(),
        poseidon_hash: dkim_hash_bz.to_vec(),
        pagination: None,
    };
    let query_bz = query.to_bytes()?;
    let query_response = deps.querier.query_grpc(
        String::from("/xion.dkim.v1.Query/QueryDkimPubKeys"),
        Binary::new(query_bz),
    )?;
    let query_response = QueryDkimPubKeysResponse::decode(query_response.as_slice())?;
    if query_response.dkim_pub_keys.is_empty() {
        return Err(InvalidDkim);
    }

    // verify the dkim pubkey hash in the proof output. the poseidon hash is
    // from the tx, we can't be sure if it was properly formatted
    inputs[2] = GrothFp::deserialize_compressed(dkim_hash_bz)?;

    let verified = GrothBn::verify(&vkey, inputs.as_slice(), &proof)?;

    Ok(verified)
}

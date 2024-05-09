use ark_bn254::{Bn254, Config, FrConfig};
use cosmwasm_std::Binary;
use crate::error::ContractResult;
use ark_groth16::{Groth16, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize};
use ark_ec::bn::Bn;
use ark_circom::CircomReduction;
use ark_crypto_primitives::snark::SNARK;
use ark_ff::Fp;
use ark_ff::MontBackend;

pub type GrothBnVkey = VerifyingKey<Bn254>;
pub type GrothBnProof = Proof<Bn<Config>>;
pub type GrothBn = Groth16<Bn254, CircomReduction>;
pub type GrothFp = Fp<MontBackend<FrConfig, 4>, 4>;


// pub fn verify(
//     sig_bytes: &Binary,
//     vkey_bytes: &Binary,
//     public_inputs: Vec<Binary>,
// ) -> ContractResult<bool> {
//     // vkey serialization is checked on submission
//     let vkey = GrothBnVkey::deserialize_compressed_unchecked(vkey_bytes.as_slice())?;
//     // proof submission is from the tx, we can't be sure if it was properly serialized
//     let proof = GrothBnProof::deserialize_compressed(sig_bytes.as_slice())?;
//     
//     // always set the hash of the msg as the first input
//     let mut inputs : Vec<GrothFp> = Vec::with_capacity(public_inputs.len() + 1);
//     
//     
//     for (i, input) in public_inputs.iter().enumerate() {
//         let field = GrothFp::deserialize_compressed_unchecked(input.as_slice())?;
//         inputs.insert(i + 1, field);
//     }
//     
//     let verified = GrothBn::verify(&vkey, inputs.as_slice(), &proof)?;
// 
//     Ok(verified)
// }
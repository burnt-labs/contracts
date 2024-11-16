use ark_bn254::{Bn254, Config, FrConfig};
use ark_circom::CircomReduction;
use ark_ec::bn::Bn;
use ark_ff::Fp;
use ark_ff::MontBackend;
use ark_groth16::{Groth16, Proof, VerifyingKey};

pub type GrothBnVkey = VerifyingKey<Bn254>;
pub type GrothBnProof = Proof<Bn<Config>>;
pub type GrothBn = Groth16<Bn254, CircomReduction>;
pub type GrothFp = Fp<MontBackend<FrConfig, 4>, 4>;

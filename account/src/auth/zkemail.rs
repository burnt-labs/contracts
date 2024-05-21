use crate::auth::groth16::{GrothBn, GrothBnProof, GrothBnVkey, GrothFp};
use crate::error::ContractResult;
use crate::proto::XionCustomQuery;
use ark_crypto_primitives::snark::SNARK;
use ark_ff::{PrimeField, Zero};
use ark_serialize::CanonicalDeserialize;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::QueryRequest::Stargate;
use cosmwasm_std::{to_binary, Binary, Deps};

const TX_BODY_MAX_BYTES: usize = 512;

fn pad_bytes(bytes: &[u8], length: usize) -> Vec<u8> {
    let mut padded = bytes.to_vec();
    let padding = length - bytes.len();
    for _ in 0..padding {
        padded.push(0);
    }
    padded
}

fn pack_bytes_into_fields(bytes: Vec<u8>) -> Vec<GrothFp> {
    // convert each 31 bytes into one field element
    let mut fields = vec![];
    bytes.chunks(31).for_each(|chunk| {
        fields.push(GrothFp::from_le_bytes_mod_order(&chunk));
    });
    fields
}
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

#[cw_serde]
struct QueryDomainHashRequest {
    domain: String,
    format: String,
}

pub fn verify(
    deps: Deps<XionCustomQuery>,
    tx_bytes: &Binary,
    sig_bytes: &Binary,
    vkey_bytes: &Binary,
    email_hash: &Binary,
    email_domain: &String,
) -> ContractResult<bool> {
    // vkey serialization is checked on submission
    let vkey = GrothBnVkey::deserialize_compressed_unchecked(vkey_bytes.as_slice())?;
    // proof submission is from the tx, we can't be sure if it was properly serialized
    let proof = GrothBnProof::deserialize_compressed(sig_bytes.as_slice())?;

    // inputs are tx body, email hash, and dmarc key hash
    let mut inputs: [GrothFp; 3] = [GrothFp::zero(); 3];

    // tx body input
    let tx_input = calculate_tx_body_commitment(STANDARD_NO_PAD.encode(tx_bytes).as_str());
    inputs[0] = tx_input;

    // email hash input, compressed at authenticator registration
    let email_hash_input = GrothFp::deserialize_compressed_unchecked(email_hash.as_slice())?;
    inputs[1] = email_hash_input;

    // dns key hash input
    // todo
    let query = QueryDomainHashRequest {
        domain: email_domain.into(),
        format: "poseidon".into(),
    };
    let query_bz = to_binary(&query)?;
    deps.querier.query(&Stargate {
        path: "xion.v1.Query/EmailDomainPubkeyHash".to_string(),
        data: query_bz,
    })?;

    let verified = GrothBn::verify(&vkey, inputs.as_slice(), &proof)?;

    Ok(verified)
}

#[cfg(test)]
mod tests {
    use crate::auth::groth16::{GrothBnProof, GrothBnVkey, GrothFp};
    use crate::auth::zkemail::{pack_bytes_into_fields, pad_bytes};
    use crate::auth::Authenticator::ZKEmail;
    use crate::proto::mock_custom_dependencies;
    use crate::testing::mock_dependencies_with_custom_querier;
    use ark_bn254::{Fq2, G1Affine, G2Affine};
    use ark_ff::Fp;
    use ark_groth16::{Proof, VerifyingKey};
    use ark_serialize::CanonicalSerialize;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::Binary;
    use serde::Deserialize;
    use std::fs;
    use std::ops::Deref;
    use std::str::FromStr;

    const EMAIL_MAX_BYTES: usize = 256;

    pub fn calculate_email_commitment(salt: &str, email: &str) -> GrothFp {
        let padded_salt_bytes = pad_bytes(salt.as_bytes(), 31);
        let padded_email_bytes = pad_bytes(email.as_bytes(), EMAIL_MAX_BYTES);
        let mut salt = pack_bytes_into_fields(padded_salt_bytes);
        let email = pack_bytes_into_fields(padded_email_bytes);
        salt.extend(email);
        let poseidon = poseidon_ark::Poseidon::new();
        poseidon.hash(salt).unwrap()
    }

    #[derive(Debug, Deserialize)]
    struct SnarkJsProof {
        pi_a: [String; 3],
        pi_b: [[String; 2]; 3],
        pi_c: [String; 3],
    }

    #[derive(Debug, Deserialize)]
    struct SnarkJsVkey {
        vk_alpha_1: [String; 3],
        vk_beta_2: [[String; 2]; 3],
        vk_gamma_2: [[String; 2]; 3],
        vk_delta_2: [[String; 2]; 3],
        IC: Vec<[String; 3]>,
    }

    #[derive(Debug)]
    pub struct PublicInputs<const N: usize> {
        inputs: [GrothFp; N],
    }

    pub trait JsonDecoder {
        fn from_json(json: &str) -> Self;
        fn from_json_file(file_path: &str) -> Self
        where
            Self: Sized,
        {
            let json = fs::read_to_string(file_path).unwrap();
            Self::from_json(&json)
        }
    }

    impl JsonDecoder for GrothBnProof {
        fn from_json(json: &str) -> Self {
            let snarkjs_proof: SnarkJsProof = serde_json::from_str(json).unwrap();
            let a = G1Affine {
                x: Fp::from_str(snarkjs_proof.pi_a[0].as_str()).unwrap(),
                y: Fp::from_str(snarkjs_proof.pi_a[1].as_str()).unwrap(),
                infinity: false,
            };
            let b = G2Affine {
                x: Fq2::new(
                    Fp::from_str(snarkjs_proof.pi_b[0][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_proof.pi_b[0][1].as_str()).unwrap(),
                ),
                y: Fq2::new(
                    Fp::from_str(snarkjs_proof.pi_b[1][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_proof.pi_b[1][1].as_str()).unwrap(),
                ),
                infinity: false,
            };
            let c = G1Affine {
                x: Fp::from_str(snarkjs_proof.pi_c[0].as_str()).unwrap(),
                y: Fp::from_str(snarkjs_proof.pi_c[1].as_str()).unwrap(),
                infinity: false,
            };
            Proof { a, b, c }
        }
    }

    impl JsonDecoder for GrothBnVkey {
        fn from_json(json: &str) -> Self {
            let snarkjs_vkey: SnarkJsVkey = serde_json::from_str(json).unwrap();
            let vk_alpha_1 = G1Affine {
                x: Fp::from_str(snarkjs_vkey.vk_alpha_1[0].as_str()).unwrap(),
                y: Fp::from_str(snarkjs_vkey.vk_alpha_1[1].as_str()).unwrap(),
                infinity: false,
            };
            let vk_beta_2 = G2Affine {
                x: Fq2::new(
                    Fp::from_str(snarkjs_vkey.vk_beta_2[0][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_vkey.vk_beta_2[0][1].as_str()).unwrap(),
                ),
                y: Fq2::new(
                    Fp::from_str(snarkjs_vkey.vk_beta_2[1][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_vkey.vk_beta_2[1][1].as_str()).unwrap(),
                ),
                infinity: false,
            };
            let vk_gamma_2 = G2Affine {
                x: Fq2::new(
                    Fp::from_str(snarkjs_vkey.vk_gamma_2[0][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_vkey.vk_gamma_2[0][1].as_str()).unwrap(),
                ),
                y: Fq2::new(
                    Fp::from_str(snarkjs_vkey.vk_gamma_2[1][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_vkey.vk_gamma_2[1][1].as_str()).unwrap(),
                ),
                infinity: false,
            };
            let vk_delta_2 = G2Affine {
                x: Fq2::new(
                    Fp::from_str(snarkjs_vkey.vk_delta_2[0][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_vkey.vk_delta_2[0][1].as_str()).unwrap(),
                ),
                y: Fq2::new(
                    Fp::from_str(snarkjs_vkey.vk_delta_2[1][0].as_str()).unwrap(),
                    Fp::from_str(snarkjs_vkey.vk_delta_2[1][1].as_str()).unwrap(),
                ),
                infinity: false,
            };

            let ic = snarkjs_vkey
                .IC
                .iter()
                .map(|ic| G1Affine {
                    x: Fp::from_str(ic[0].as_str()).unwrap(),
                    y: Fp::from_str(ic[1].as_str()).unwrap(),
                    infinity: false,
                })
                .collect();

            VerifyingKey {
                alpha_g1: vk_alpha_1,
                beta_g2: vk_beta_2,
                gamma_g2: vk_gamma_2,
                delta_g2: vk_delta_2,
                gamma_abc_g1: ic,
            }
        }
    }

    impl<const N: usize> JsonDecoder for PublicInputs<N> {
        fn from_json(json: &str) -> Self {
            let inputs: Vec<String> = serde_json::from_str(json).unwrap();
            let inputs: Vec<GrothFp> = inputs
                .iter()
                .map(|input| Fp::from_str(input).unwrap())
                .collect();
            Self {
                inputs: inputs.try_into().unwrap(),
            }
        }
    }

    impl<const N: usize> PublicInputs<N> {
        pub fn from(inputs: [&str; N]) -> Self {
            let inputs: Vec<GrothFp> = inputs
                .iter()
                .map(|input| Fp::from_str(input).unwrap())
                .collect();
            Self {
                inputs: inputs.try_into().unwrap(),
            }
        }
    }

    impl<const N: usize> Deref for PublicInputs<N> {
        type Target = [GrothFp];

        fn deref(&self) -> &Self::Target {
            &self.inputs
        }
    }

    #[test]
    fn should_verify_body_proof() {
        assert_verification(
            "src/auth/tests/data/body/vkey.json",
            "src/auth/tests/data/body/proof.json",
            "src/auth/tests/data/body/public.json",
        );
    }

    #[test]
    fn should_verify_header_proof() {
        assert_verification(
            "src/auth/tests/data/subject/vkey.json",
            "src/auth/tests/data/subject/proof.json",
            "src/auth/tests/data/subject/public.json",
        );
    }

    fn assert_verification(
        vkey_json_path: &str,
        proof_json_path: &str,
        public_inputs_json_path: &str,
    ) {
        const SALT: &str = "XRhMS5Nc2dTZW5kEpAB";
        const EMAIL: &str = "thezdev1@gmail.com";
        const TX: &str = "CrQBCrEBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEpABCj94aW9uMWd2cDl5djZndDBwcmdzc3\
        ZueWNudXpnZWszZmtyeGxsZnhxaG0wNzYwMmt4Zmc4dXI2NHNuMnAycDkSP3hpb24xNGNuMG40ZjM4ODJzZ3B2NWQ5ZzA2dzNxN3hzZ\
        m51N3B1enltZDk5ZTM3ZHAwemQ4bTZscXpwemwwbRoMCgV1eGlvbhIDMTAwEmEKTQpDCh0vYWJzdHJhY3RhY2NvdW50LnYxLk5pbFB1\
        YktleRIiCiBDAlIzSFvCNEIMmTE+CRm0U2Gb/0mBfb/aeqxkoPweqxIECgIIARh/EhAKCgoFdXhpb24SATAQwJoMGg54aW9uLXRlc3R\
        uZXQtMSCLjAo=";

        let vkey = GrothBnVkey::from_json_file(vkey_json_path);
        let mut vkey_serialized = Vec::new();
        vkey.serialize_compressed(&mut vkey_serialized).unwrap();
        let proof = GrothBnProof::from_json_file(proof_json_path);
        let mut proof_serialized = Vec::new();
        proof.serialize_compressed(&mut proof_serialized).unwrap();

        let public_inputs: PublicInputs<3> = PublicInputs::from_json_file(public_inputs_json_path);

        let mut domain_key_hash = Vec::new();
        public_inputs.inputs[2]
            .serialize_compressed(&mut domain_key_hash)
            .unwrap();

        let email_hash = calculate_email_commitment(SALT, EMAIL);
        let mut email_hash_serialized = Vec::new();
        email_hash
            .serialize_compressed(&mut email_hash_serialized)
            .unwrap();

        let authenticator = ZKEmail {
            vkey: Binary::from(vkey_serialized),
            email_hash: Binary::from(email_hash_serialized),
            email_domain: "gmail.com".to_string(),
        };

        let mut deps = mock_dependencies_with_custom_querier(&[]);
        let env = mock_env();
        let query = QueryDomainHashRequest {
            domain: "gmail.com".into(),
            format: "poseidon".into(),
        };
        let query_bz = to_binary(&query)?;
        //
        // querier.handle_query(&Stargate {
        //     path: "xion.v1.Query/EmailDomainPubkeyHash".to_string(),
        //     data: query_bz,
        // });
        // querier.with_custom_handler(() => )
        // deps.querier = querier;

        let result = authenticator.verify(
            deps.as_ref(),
            &env.clone(),
            &Binary::from_base64(TX).unwrap(),
            &Binary::from(proof_serialized),
        );
        assert!(result.unwrap())

        // let verified = GrothBn::verify(&vkey, &public_inputs, &proof).unwrap();
        // let email_commitment = calculate_email_commitment(SALT, EMAIL);
        // let tx_body_commitment = calculate_tx_body_commitment(TX);
        //
        // assert!(verified);
        // assert_eq!(public_inputs[0], tx_body_commitment);
        // assert_eq!(public_inputs[1], email_commitment);
    }
}

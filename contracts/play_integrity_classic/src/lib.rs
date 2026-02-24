#[cfg(not(feature = "library"))]
pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::{STANDARD as B64, URL_SAFE_NO_PAD};
    use base64::Engine as _;
    use p256::ecdsa::{signature::Signer, signature::Verifier, SigningKey, VerifyingKey};
    use p256::pkcs8::DecodePublicKey;

    use crate::msg::IntegrityVerdict;

    // Original test data from app_attest_android — the verification key (SPKI DER,
    // base64-standard) and the JWE integrity token. The JWE decryption key and token
    // are preserved here for reference; the relayer would decrypt the JWE off-chain
    // to obtain the inner JWS before submitting it on-chain.
    #[allow(dead_code)]
    const DECRYPTION_KEY: &str = "lhzLdY8Ap3h5VvEuaTu0A1v3VGCPx6Agd0ORlN1BGCw=";
    const VERIFICATION_KEY_SPKI: &str = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEehNYmCDWCZAGSRLtOHOx6xeuoaWZlpavBRXRpeI3apBrmSDUBYsftwUXmGZh5lLlKeO3yTZOZgDnEq8Mu0t5+w==";
    #[allow(dead_code)]
    const INTEGRITY_TOKEN_JWE: &str = "eyJhbGciOiJBMjU2S1ciLCJlbmMiOiJBMjU2R0NNIn0.Umc2gHxcsCqrSHLmjwmH0vsRGgTY_dZJ56QQxpAk_S8h0Dn8imbVAw.qfzNXdKNon8eW_BI.-vZ7AI8PeHpAwytVQQ-3UAjWQCz5R_4H7pjLYMJqLB_UlbYwhLXzFhrF9QkX9I21emwTsevqmYhLgMjC4TsgTTjTGTYUzQiRFlBzFoQ_bh0-jjJddsmNE5WRzXpZQuhalWrhEACthop7iUxLk4oCPGtmNA7ezma_sadfxXr2U31kqhuw_DRn_jSJoYLM85oambqYIRMwF0xJr4EYQdWP_f1J2OqT7gI4U4YtoH-FKMaNg4JvWR00E7wUzi_3nyu5XGkWLsmmC2qxLBpB644-2Qwvd2K9A022Eh7fb4GOi-lXW9u-L5IEXsOuQjxvv-FsLvu-SX99qDhaQ8DfUZT9aKHu7T7ZyB6254-BUzFPCVgxCbyq5LD4RbLxBsXpmMd7Qfdq9syqJ8W6s3250JuY_yCtUXRDQQo_UNZWf6rxiNh_mINz7OQFwLduqlvV-qX0O5jZCwElgv9QFU_nbJMzdnGgCkDcLGXkQLnLOvOSeRnDMxFrbqruho0oXeZCUMDRKAGa92MQoYQMhEKWwvOsoomIPAP5OsKHQtzGKljYHAx2T9cFLWQZjgdLkmoMTE8p0D1VLIf6Sa8DyG66BJG3hKLLJ9jYuE0a3cYfcmkTpDyCVPj-7mLuQoRkj8QeZFiHRE8iZWoXRU9KE6_wfPYBybkhWXf0kSHZINzdipnshsc7EeM_ErarQZVazARa5EZXw-qM3R5xUvAGVzM8uyHlaL0dtE0KPoCCWhp5mO01kwnD28QhHYT6at3kARHMCFplXR3MDKprCRJFQFpTKLb8Cbz41W3-Yd4mgeKciOe_DoYpPhi2jMT1mbY5IcniP0gXNMKfShZswxIaIOK0PyQM2gd1Tc6jGsE_ApGnvmgStCxXCtR5Sbk8kLrqGUJ8jkVa40wX5aH3c8jymbQwqbXtuVbDsDYtY5IgWuZD0spukCfHfaolwHQZOZAOYkx2IKoGJetIMQBKGYF3z6Jy31kw65Vg6qpTb9-x__0vqtHffy24Sx5j3eQVQ7rjFysnEpFEI93Pn5tQIMlgVdGRYlz8M0a7V_HgY-C05lGwgVe5aeuzZgOg7qUlESghV1S96TRJ4F1AMwKvXvga4_12qtj2sOIvYR1zFzRhUw005jpqCbm867nBoEKs6wGnFNKQ91WPVXy2mMA3eM5L0jSp2HCAb_KGFKFt_zyFebFv6hP96H1bo-ebt3gQape4lFoDYc1QCs6QZg.q8WYiDAj7O9nETLw62NwJw";

    /// A realistic Play Integrity verdict payload matching Google's format.
    fn sample_verdict_json() -> &'static str {
        r#"{"requestDetails":{"requestPackageName":"com.burnt.xion","nonce":"dGVzdC1ub25jZQ","timestampMillis":"1700000000000"},"appIntegrity":{"appRecognitionVerdict":"PLAY_RECOGNIZED","packageName":"com.burnt.xion","certificateSha256Digest":["abc123"],"versionCode":"42"},"deviceIntegrity":{"deviceRecognitionVerdict":["MEETS_DEVICE_INTEGRITY"]},"accountDetails":{"appLicensingVerdict":"LICENSED"}}"#
    }

    /// Build a compact JWS (ES256) from a signing key and JSON payload.
    fn build_test_jws(signing_key: &SigningKey, payload_json: &str) -> String {
        let header = r#"{"alg":"ES256","typ":"JWT"}"#;
        let header_b64 = URL_SAFE_NO_PAD.encode(header.as_bytes());
        let payload_b64 = URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let signature: p256::ecdsa::Signature = signing_key.sign(signing_input.as_bytes());
        let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());
        format!("{}.{}.{}", header_b64, payload_b64, sig_b64)
    }

    /// Verify a compact JWS and return the payload bytes.
    /// Used by the mock querier to mirror xion's VerifyJWS behavior.
    fn verify_jws_payload(compact_jws: &str, verification_key: &[u8]) -> Result<Vec<u8>, String> {
        let parts: Vec<&str> = compact_jws.splitn(3, '.').collect();
        if parts.len() != 3 {
            return Err("invalid JWS: expected 3 dot-separated parts".into());
        }
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let sig_bytes = URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|e| format!("base64 decode error: {e}"))?;
        let signature = p256::ecdsa::Signature::from_bytes(sig_bytes.as_slice().into())
            .map_err(|e| format!("invalid signature: {e}"))?;
        let vk = VerifyingKey::from_sec1_bytes(verification_key)
            .map_err(|e| format!("invalid key: {e}"))?;
        vk.verify(signing_input.as_bytes(), &signature)
            .map_err(|e| format!("signature verification failed: {e}"))?;
        let payload = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| format!("base64 decode error: {e}"))?;
        Ok(payload)
    }

    #[test]
    fn test_spki_key_parses() {
        let der = B64.decode(VERIFICATION_KEY_SPKI).unwrap();
        let pubkey = p256::PublicKey::from_public_key_der(&der).unwrap();
        let sec1 = pubkey.to_sec1_bytes();
        assert_eq!(sec1.len(), 65);
        assert_eq!(sec1[0], 0x04);
    }

    #[test]
    fn test_verify_jws() {
        let signing_key = SigningKey::random(&mut rand_core::OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);
        let sec1_bytes = verifying_key.to_sec1_bytes();

        let jws = build_test_jws(&signing_key, sample_verdict_json());
        let payload = verify_jws_payload(&jws, &sec1_bytes).unwrap();
        let verdict: IntegrityVerdict = serde_json::from_slice(&payload).unwrap();

        assert_eq!(verdict.request_details.request_package_name, "com.burnt.xion");
        assert_eq!(verdict.request_details.nonce, "dGVzdC1ub25jZQ");
        assert_eq!(verdict.app_integrity.app_recognition_verdict, "PLAY_RECOGNIZED");
        assert_eq!(verdict.app_integrity.package_name.as_deref(), Some("com.burnt.xion"));
        assert_eq!(
            verdict.device_integrity.device_recognition_verdict,
            vec!["MEETS_DEVICE_INTEGRITY"]
        );
        assert_eq!(verdict.account_details.app_licensing_verdict, "LICENSED");
    }

    #[test]
    fn test_wrong_key_rejected() {
        let signing_key = SigningKey::random(&mut rand_core::OsRng);
        let other_key = SigningKey::random(&mut rand_core::OsRng);
        let wrong_pubkey = VerifyingKey::from(&other_key).to_sec1_bytes();

        let jws = build_test_jws(&signing_key, sample_verdict_json());
        let result = verify_jws_payload(&jws, &wrong_pubkey);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_with_spki_derived_key() {
        let der = B64.decode(VERIFICATION_KEY_SPKI).unwrap();
        let pubkey = p256::PublicKey::from_public_key_der(&der).unwrap();
        let sec1 = pubkey.to_sec1_bytes();

        let verifying_key = VerifyingKey::from_sec1_bytes(&sec1).unwrap();
        assert_eq!(verifying_key.to_sec1_bytes().len(), 65);
    }

    // ── End-to-end flow test ────────────────────────────────────────────

    mod e2e {
        use super::*;
        use crate::contract;
        use crate::msg::{InstantiateMsg, QueryMsg, VerifyResponse};
        use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
        use cosmwasm_std::{
            from_json, Binary, ContractResult, Empty, GrpcQuery, OwnedDeps, Querier,
            QuerierResult, QueryRequest, SystemError, SystemResult,
        };
        use josekit::jwe::{self, JweHeader, A256KW};
        use prost::Message;

        /// Same proto types the contract defines internally.
        #[derive(Clone, PartialEq, ::prost::Message)]
        struct QueryVerifyJwsRequest {
            #[prost(string, tag = "1")]
            pub aud: String,
            #[prost(string, tag = "2")]
            pub sig_bytes: String,
        }

        #[derive(Clone, PartialEq, ::prost::Message)]
        struct QueryVerifyJwsResponse {
            #[prost(bytes = "vec", tag = "1")]
            pub payload: Vec<u8>,
        }

        /// A custom querier that mocks xion's JWK module VerifyJWS endpoint.
        /// It performs real ES256 signature verification using the stored key.
        struct JwkMockQuerier {
            base: MockQuerier<Empty>,
            /// SEC1-encoded P-256 public key for signature verification.
            verification_key: Vec<u8>,
        }

        impl Querier for JwkMockQuerier {
            fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
                let request: QueryRequest<Empty> = match from_json(bin_request) {
                    Ok(v) => v,
                    Err(e) => {
                        return SystemResult::Err(SystemError::InvalidRequest {
                            error: format!("Parsing query request: {e}"),
                            request: bin_request.into(),
                        })
                    }
                };

                match &request {
                    QueryRequest::Grpc(GrpcQuery { path, data }) => {
                        self.handle_grpc(path, data)
                    }
                    _ => self.base.handle_query(&request),
                }
            }
        }

        impl JwkMockQuerier {
            fn handle_grpc(&self, path: &str, data: &Binary) -> QuerierResult {
                if path != "/xion.jwk.v1.Query/VerifyJWS" {
                    return SystemResult::Err(SystemError::UnsupportedRequest {
                        kind: format!("Unexpected gRPC path: {path}"),
                    });
                }

                let req = match QueryVerifyJwsRequest::decode(data.as_slice()) {
                    Ok(r) => r,
                    Err(e) => {
                        return SystemResult::Ok(ContractResult::Err(format!(
                            "Failed to decode VerifyJWS request: {e}"
                        )));
                    }
                };

                // Actually verify the ES256 signature — the mock does real crypto,
                // mirroring what the on-chain JWK module does.
                match verify_jws_payload(&req.sig_bytes, &self.verification_key) {
                    Ok(payload) => {
                        let resp = QueryVerifyJwsResponse { payload };
                        SystemResult::Ok(ContractResult::Ok(Binary::new(resp.encode_to_vec())))
                    }
                    Err(e) => SystemResult::Ok(ContractResult::Err(format!(
                        "signature verification failed: {e}"
                    ))),
                }
            }
        }

        fn mock_deps_with_verify_jws(
            verification_key: Vec<u8>,
        ) -> OwnedDeps<MockStorage, MockApi, JwkMockQuerier> {
            OwnedDeps {
                storage: MockStorage::default(),
                api: MockApi::default(),
                querier: JwkMockQuerier {
                    base: MockQuerier::default(),
                    verification_key,
                },
                custom_query_type: std::marker::PhantomData,
            }
        }

        /// Full end-to-end flow:
        ///   1. Generate keypair (simulating Google's signing key)
        ///   2. Sign a verdict payload → compact JWS
        ///   3. Encrypt the JWS into a JWE (simulating Google's token format)
        ///   4. Decrypt the JWE (simulating the relayer)
        ///   5. Query the contract with the JWS
        ///   6. Mock JWK module verifies ES256, returns payload
        ///   7. Contract parses and returns the verdict
        #[test]
        fn test_end_to_end_jwe_to_verdict() {
            // ── 1. Generate a test P-256 keypair ──
            let signing_key = SigningKey::random(&mut rand_core::OsRng);
            let verifying_key = VerifyingKey::from(&signing_key);
            let sec1_bytes = verifying_key.to_sec1_bytes().to_vec();

            // ── 2. Build a JWS with a realistic verdict ──
            let compact_jws = build_test_jws(&signing_key, sample_verdict_json());

            // ── 3. Encrypt the JWS into a JWE (A256KW + A256GCM) ──
            let aes_key = [0x42u8; 32]; // test AES-256 key-encryption-key
            let encrypter = A256KW.encrypter_from_bytes(&aes_key).unwrap();
            let mut jwe_header = JweHeader::new();
            jwe_header.set_content_encryption("A256GCM");
            let jwe_token =
                jwe::serialize_compact(compact_jws.as_bytes(), &jwe_header, &encrypter).unwrap();

            // Verify JWE has 5 dot-separated parts
            assert_eq!(jwe_token.split('.').count(), 5);

            // ── 4. Decrypt the JWE (relayer's off-chain work) ──
            let decrypter = A256KW.decrypter_from_bytes(&aes_key).unwrap();
            let (plaintext, _header) = jwe::deserialize_compact(&jwe_token, &decrypter).unwrap();
            let decrypted_jws = String::from_utf8(plaintext).unwrap();
            assert_eq!(decrypted_jws, compact_jws);

            // ── 5. Set up contract with mock JWK module ──
            let mut deps = mock_deps_with_verify_jws(sec1_bytes);

            let info = message_info(&cosmwasm_std::Addr::unchecked("relayer"), &[]);
            let init_msg = InstantiateMsg {
                aud: "play-integrity-test".to_string(),
            };
            contract::instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

            // ── 6. Query the contract with the JWS ──
            let query_msg = QueryMsg::Verify {
                compact_jws: decrypted_jws,
            };
            let res = contract::query(deps.as_ref(), mock_env(), query_msg).unwrap();

            // ── 7. Verify the verdict in the response ──
            let resp: VerifyResponse = from_json(res).unwrap();
            assert_eq!(
                resp.verdict.request_details.request_package_name,
                "com.burnt.xion"
            );
            assert_eq!(resp.verdict.request_details.nonce, "dGVzdC1ub25jZQ");
            assert_eq!(
                resp.verdict.request_details.timestamp_millis,
                "1700000000000"
            );
            assert_eq!(
                resp.verdict.app_integrity.app_recognition_verdict,
                "PLAY_RECOGNIZED"
            );
            assert_eq!(
                resp.verdict.app_integrity.package_name.as_deref(),
                Some("com.burnt.xion")
            );
            assert_eq!(
                resp.verdict.device_integrity.device_recognition_verdict,
                vec!["MEETS_DEVICE_INTEGRITY"]
            );
            assert_eq!(
                resp.verdict.account_details.app_licensing_verdict,
                "LICENSED"
            );
        }

        /// Verify that a JWS signed with the wrong key is rejected.
        #[test]
        fn test_end_to_end_wrong_key_rejected() {
            let signing_key = SigningKey::random(&mut rand_core::OsRng);
            let wrong_key = SigningKey::random(&mut rand_core::OsRng);
            let wrong_pubkey = VerifyingKey::from(&wrong_key).to_sec1_bytes().to_vec();

            let compact_jws = build_test_jws(&signing_key, sample_verdict_json());

            // Set up contract with the WRONG verification key
            let mut deps = mock_deps_with_verify_jws(wrong_pubkey);
            let info = message_info(&cosmwasm_std::Addr::unchecked("relayer"), &[]);
            contract::instantiate(
                deps.as_mut(),
                mock_env(),
                info,
                InstantiateMsg {
                    aud: "test".to_string(),
                },
            )
            .unwrap();

            let query_msg = QueryMsg::Verify { compact_jws };
            let err = contract::query(deps.as_ref(), mock_env(), query_msg).unwrap_err();
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("signature verification failed"),
                "Expected signature error, got: {err_msg}"
            );
        }
    }
}

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use josekit::jwe::{self, JweHeader, A256KW};
use josekit::jws::{self, ES256};

pub fn decrypt_and_verify_with_josekit(
    base64_of_encoded_decryption_key: &str,
    base64_of_encoded_verification_key_x509: &str,
    integrity_token_compact_jwe: &str,
) -> Result<String> {
    // 1) AES-256 key for A256KW
    let kek = B64
        .decode(base64_of_encoded_decryption_key)
        .context("decode AES key b64")?;
    anyhow::ensure!(kek.len() == 32, "Expected 32-byte AES-256 key");

    // 2) EC public key (SPKI DER) as PEM for josekit ES256 verifier
    let der = B64
        .decode(base64_of_encoded_verification_key_x509)
        .context("decode EC pubkey b64 (SPKI DER)")?;

    // 3) Decrypt compact JWE → compact JWS using A256KW (content enc from header)
    let decrypter = A256KW
        .decrypter_from_bytes(&kek)
        .context("create A256KW decrypter")?;
    let (plaintext, _header): (Vec<u8>, JweHeader) =
        jwe::deserialize_compact(integrity_token_compact_jwe, &decrypter)
            .context("JWE compact decryption failed")?;

    let compact_jws = String::from_utf8(plaintext).context("JWE payload not UTF-8")?;

    // 4) Verify compact JWS (ES256) using the P‑256 public key (PEM)
    let verifier = ES256.verifier_from_der(der)
        .context("create ES256 verifier from PEM (SPKI)")?;

    let (payload, _jws_header) = jws::deserialize_compact(&compact_jws, &verifier)
        .context("JWS ES256 verification failed")?;

    let payload_str = String::from_utf8(payload).context("JWS payload not UTF-8")?;
    Ok(payload_str)
}

fn main() {
    let decryption_key = "lhzLdY8Ap3h5VvEuaTu0A1v3VGCPx6Agd0ORlN1BGCw=";
    let verification_key = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEehNYmCDWCZAGSRLtOHOx6xeuoaWZlpavBRXRpeI3apBrmSDUBYsftwUXmGZh5lLlKeO3yTZOZgDnEq8Mu0t5+w==";
    let integrity_token = "eyJhbGciOiJBMjU2S1ciLCJlbmMiOiJBMjU2R0NNIn0.Umc2gHxcsCqrSHLmjwmH0vsRGgTY_dZJ56QQxpAk_S8h0Dn8imbVAw.qfzNXdKNon8eW_BI.-vZ7AI8PeHpAwytVQQ-3UAjWQCz5R_4H7pjLYMJqLB_UlbYwhLXzFhrF9QkX9I21emwTsevqmYhLgMjC4TsgTTjTGTYUzQiRFlBzFoQ_bh0-jjJddsmNE5WRzXpZQuhalWrhEACthop7iUxLk4oCPGtmNA7ezma_sadfxXr2U31kqhuw_DRn_jSJoYLM85oambqYIRMwF0xJr4EYQdWP_f1J2OqT7gI4U4YtoH-FKMaNg4JvWR00E7wUzi_3nyu5XGkWLsmmC2qxLBpB644-2Qwvd2K9A022Eh7fb4GOi-lXW9u-L5IEXsOuQjxvv-FsLvu-SX99qDhaQ8DfUZT9aKHu7T7ZyB6254-BUzFPCVgxCbyq5LD4RbLxBsXpmMd7Qfdq9syqJ8W6s3250JuY_yCtUXRDQQo_UNZWf6rxiNh_mINz7OQFwLduqlvV-qX0O5jZCwElgv9QFU_nbJMzdnGgCkDcLGXkQLnLOvOSeRnDMxFrbqruho0oXeZCUMDRKAGa92MQoYQMhEKWwvOsoomIPAP5OsKHQtzGKljYHAx2T9cFLWQZjgdLkmoMTE8p0D1VLIf6Sa8DyG66BJG3hKLLJ9jYuE0a3cYfcmkTpDyCVPj-7mLuQoRkj8QeZFiHRE8iZWoXRU9KE6_wfPYBybkhWXf0kSHZINzdipnshsc7EeM_ErarQZVazARa5EZXw-qM3R5xUvAGVzM8uyHlaL0dtE0KPoCCWhp5mO01kwnD28QhHYT6at3kARHMCFplXR3MDKprCRJFQFpTKLb8Cbz41W3-Yd4mgeKciOe_DoYpPhi2jMT1mbY5IcniP0gXNMKfShZswxIaIOK0PyQM2gd1Tc6jGsE_ApGnvmgStCxXCtR5Sbk8kLrqGUJ8jkVa40wX5aH3c8jymbQwqbXtuVbDsDYtY5IgWuZD0spukCfHfaolwHQZOZAOYkx2IKoGJetIMQBKGYF3z6Jy31kw65Vg6qpTb9-x__0vqtHffy24Sx5j3eQVQ7rjFysnEpFEI93Pn5tQIMlgVdGRYlz8M0a7V_HgY-C05lGwgVe5aeuzZgOg7qUlESghV1S96TRJ4F1AMwKvXvga4_12qtj2sOIvYR1zFzRhUw005jpqCbm867nBoEKs6wGnFNKQ91WPVXy2mMA3eM5L0jSp2HCAb_KGFKFt_zyFebFv6hP96H1bo-ebt3gQape4lFoDYc1QCs6QZg.q8WYiDAj7O9nETLw62NwJw";
    let result = decrypt_and_verify_with_josekit(decryption_key, verification_key, integrity_token);
    println!("result = {:?}", result);
}

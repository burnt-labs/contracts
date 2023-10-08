use crate::error::ContractError::{InvalidJWTAud, InvalidTime, InvalidToken};
use crate::error::ContractResult;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use cosmwasm_std::Timestamp;
use phf::{phf_map, Map};
use rsa::traits::SignatureScheme;
use rsa::{BigUint, Pkcs1v15Sign, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str;

static AUD_KEY_MAP: Map<&'static str, &'static str> = phf_map! {
    "project-test-185e9a9f-8bab-42f2-a924-953a59e8ff94" => "sQKkA829tzjU2VA-INHvdrewkbQzjpsMn0PNM7KJaBODbB4ItZM4x1NVSWBiy2DGHkaDDvADRbbq1BZsC1iXVtIYm0AoD7x4QC1w89kp2_s0wmvUOSPiQZlYrgJqRDXirXJZX3MNku2McXbwdyPajDaR4nBBQOoUOF21CHqLDqBHs2R6tHyL80R_8mgueiqQ-4wg6SSVcB_6ZOh59vRcjKr34upKPWGQzvMGCkeTO9whzbIWbA1j-8ykiS63EhjWBZU_sSolsf1ZGq8peVrADDLhOvHtZxCZLKwB46k2kb8GKAWlO4wRP6BDVjzpnea7BsvZ6JwULKg3HisH9gzaiQ;AQAB",
};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    aud: Box<[String]>, // Optional. Audience
    exp: u64, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: u64, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    nbf: u64, // Optional. Not Before (as UTC timestamp)
    sub: String, // Optional. Subject (whom token refers to)

    transaction_hash: String,
}

pub fn verify(
    current_time: &Timestamp,
    tx_hash: &Vec<u8>,
    sig_bytes: &[u8],
    aud: &str,
    sub: &str,
) -> ContractResult<bool> {
    if !AUD_KEY_MAP.contains_key(aud) {
        return Err(InvalidJWTAud);
    }

    let key = match AUD_KEY_MAP.get(aud) {
        None => return Err(InvalidJWTAud),
        Some(k) => *k,
    };

    // prepare the components of the token for verification
    let mut components = sig_bytes.split(|&b| b == b'.');
    let header_bytes = components.next().ok_or(InvalidToken)?; // ignore the header, it is not currently used
    let payload_bytes = components.next().ok_or(InvalidToken)?;
    let digest_bytes = [header_bytes, &[b'.'], payload_bytes].concat();
    let signature_bytes = components.next().ok_or(InvalidToken)?;
    let signature = URL_SAFE_NO_PAD.decode(signature_bytes)?;

    // retrieve and rebuild the pubkey
    let mut key_split = key.split(';');
    let modulus = key_split.next().ok_or(InvalidJWTAud)?;
    let mod_bytes = URL_SAFE_NO_PAD.decode(modulus)?;
    let exponent = key_split.next().ok_or(InvalidJWTAud)?;
    let exp_bytes = URL_SAFE_NO_PAD.decode(exponent)?;
    let pubkey = RsaPublicKey::new(
        BigUint::from_bytes_be(mod_bytes.as_slice()),
        BigUint::from_bytes_be(exp_bytes.as_slice()),
    )?;

    // hash the message body before verification
    let mut hasher = Sha256::new();
    hasher.update(digest_bytes);
    let digest = hasher.finalize().as_slice().to_vec();

    // verify the signature
    let scheme = Pkcs1v15Sign::new::<Sha256>();
    scheme.verify(&pubkey, digest.as_slice(), signature.as_slice())?;

    // at this point, we have verified that the token is legitimately signed.
    // now we perform logic checks on the body
    let payload = URL_SAFE_NO_PAD.decode(payload_bytes)?;
    let claims: Claims = cosmwasm_std::from_slice(payload.as_slice())?;
    if !claims.sub.eq_ignore_ascii_case(sub) {
        // this token was not for the supplied sub
        return Err(InvalidToken);
    }
    if !claims.aud.contains(&aud.to_string()) {
        // this token was for a different aud
        return Err(InvalidToken);
    }

    // complete the time checks
    let expiration = Timestamp::from_seconds(claims.exp as u64);
    if expiration.lt(current_time) {
        return Err(InvalidTime);
    }
    let not_before = Timestamp::from_seconds(claims.nbf as u64);
    if not_before.gt(current_time) {
        return Err(InvalidTime);
    }
    // make sure the provided hash matches the one from the tx
    Ok(tx_hash.eq(claims.transaction_hash.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_token() {
        let encoded_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6Imp3ay10ZXN0LWI5OGFjMTExLTg1MTUtNGY0OS05MDU2LTdmM2E5NzJmNzU4MSIsInR5cCI6IkpXVCJ9.eyJhdWQiOlsicHJvamVjdC10ZXN0LTE4NWU5YTlmLThiYWItNDJmMi1hOTI0LTk1M2E1OWU4ZmY5NCJdLCJleHAiOjE2OTU4NDgxNjksImh0dHBzOi8vc3R5dGNoLmNvbS9zZXNzaW9uIjp7ImlkIjoic2Vzc2lvbi10ZXN0LTk1MDUyMzJkLTczNjUtNDExZC1hYzBlLTc3MWM2YWY2MmU3NCIsInN0YXJ0ZWRfYXQiOiIyMDIzLTA5LTI3VDIwOjUxOjA5WiIsImxhc3RfYWNjZXNzZWRfYXQiOiIyMDIzLTA5LTI3VDIwOjUxOjA5WiIsImV4cGlyZXNfYXQiOiIyMDIzLTEwLTI3VDIwOjUxOjA5WiIsImF0dHJpYnV0ZXMiOnsidXNlcl9hZ2VudCI6IiIsImlwX2FkZHJlc3MiOiIifSwiYXV0aGVudGljYXRpb25fZmFjdG9ycyI6W3sidHlwZSI6Im90cCIsImRlbGl2ZXJ5X21ldGhvZCI6ImVtYWlsIiwibGFzdF9hdXRoZW50aWNhdGVkX2F0IjoiMjAyMy0wOS0yN1QyMDo1MTowOVoiLCJlbWFpbF9mYWN0b3IiOnsiZW1haWxfaWQiOiJlbWFpbC10ZXN0LWY1ODU1MDU1LTc2OWMtNGVhMC04YzZhLTFmMTUyNDgzNWJmMiIsImVtYWlsX2FkZHJlc3MiOiJmaWxhbWVudCsxNjk1ODQ3ODUyMDY1QGJ1cm50LmNvbSJ9fV19LCJpYXQiOjE2OTU4NDc4NjksImlzcyI6InN0eXRjaC5jb20vcHJvamVjdC10ZXN0LTE4NWU5YTlmLThiYWItNDJmMi1hOTI0LTk1M2E1OWU4ZmY5NCIsIm5iZiI6MTY5NTg0Nzg2OSwic3ViIjoidXNlci10ZXN0LWY1MmRkZWQyLWM5M2EtNGVmZC05MTY5LTU3N2ZiZGY2Y2I4NiIsInRyYW5zYWN0aW9uX2hhc2giOiIweDEyMzQ1Njc4OTBfMyJ9.SkoLgjViIkP2kVcgz4fqA1CEWKbkN40behhs_ph-uCIAWNakCC_6FEfvcYgxBp1idk2IPRRZir-QAK_XV8ov_RHZuCqc8qd_bzlAwXV7cElHDf8oQxs44kA_P81QExoCABa3_ZzJ8KUNwzY0NbFxI3oJKDOYGxapi5aY5xsuGO3wUJX4PlsJ5xQhn254THgxpstqXEj56K1bcuDw_y-TQiNnP9R3vLZfGWj6BkZjB8dfgMp6FMRq_tBmo4l37pVHIA8v3-tlYSNoV7Z1P2uLsuxrM3_eh5zJ48vogwiOYAC2Ih90Pp2mF0leKOEXC4feTl_2oPIvGdXqbxgAmGKjpA";
        let encoded_hash = "0x1234567890_3";
        let hash_bytes = encoded_hash.as_bytes().to_vec();

        let verification = verify(
            &Timestamp::from_seconds(1695847870),
            &hash_bytes,
            encoded_token.as_bytes(),
            &"project-test-185e9a9f-8bab-42f2-a924-953a59e8ff94".to_string(),
            &"user-test-f52dded2-c93a-4efd-9169-577fbdf6cb86".to_string(),
        );
        assert!(verification.unwrap());
    }
}

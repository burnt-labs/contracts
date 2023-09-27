use crate::error::ContractError::{InvalidJWTAud, InvalidTime};
use crate::error::ContractResult;
use base64::{engine::general_purpose, Engine as _};
use cosmwasm_std::Timestamp;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use phf::{phf_map, Map};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str;

static AUD_KEY_MAP: Map<&'static str, &'static str> = phf_map! {
    "project-test-185e9a9f-8bab-42f2-a924-953a59e8ff94" => "sQKkA829tzjU2VA-INHvdrewkbQzjpsMn0PNM7KJaBODbB4ItZM4x1NVSWBiy2DGHkaDDvADRbbq1BZsC1iXVtIYm0AoD7x4QC1w89kp2_s0wmvUOSPiQZlYrgJqRDXirXJZX3MNku2McXbwdyPajDaR4nBBQOoUOF21CHqLDqBHs2R6tHyL80R_8mgueiqQ-4wg6SSVcB_6ZOh59vRcjKr34upKPWGQzvMGCkeTO9whzbIWbA1j-8ykiS63EhjWBZU_sSolsf1ZGq8peVrADDLhOvHtZxCZLKwB46k2kb8GKAWlO4wRP6BDVjzpnea7BsvZ6JwULKg3HisH9gzaiQ;AQAB",
};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    aud: Box<[String]>, // Optional. Audience
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    nbf: usize, // Optional. Not Before (as UTC timestamp)
    sub: String, // Optional. Subject (whom token refers to)

    transaction_hash: String,
}

pub fn verify(
    current_time: &Timestamp,
    tx_hash: &Vec<u8>,
    sig_bytes: &[u8],
    aud: &String,
    sub: &String,
) -> ContractResult<bool> {
    if !AUD_KEY_MAP.contains_key(aud.as_str()) {
        return Err(InvalidJWTAud);
    }

    let key = match AUD_KEY_MAP.get(aud.as_str()) {
        None => return Err(InvalidJWTAud),
        Some(k) => *k,
    };

    let token = str::from_utf8(sig_bytes)?;

    // currently only RS256 is supported
    let mut options = Validation::new(Algorithm::RS256);
    options.required_spec_claims = HashSet::from([
        "sub".to_string(),
        "aud".to_string(),
        "exp".to_string(),
        "nbf".to_string(),
        "iat".to_string(),
        "iss".to_string(),
        "transaction_hash".to_string(),
    ]);

    // make sure the sub and aud ids are as expected
    options.sub = Option::from(sub.clone());
    options.aud = Option::from(HashSet::from([aud.clone()]));

    // disable time checks because system time is not available, will pull directly from BlockInfo
    options.validate_exp = false;
    options.validate_nbf = false;

    let mut key_split = key.split(';');
    let modulus = key_split.next().ok_or(InvalidJWTAud)?;
    let exponent = key_split.next().ok_or(InvalidJWTAud)?;
    let decoding_key = DecodingKey::from_rsa_components(modulus, exponent)?;
    let token = decode::<Claims>(&token, &decoding_key, &options)?;

    // complete the time checks
    let expiration = Timestamp::from_seconds(token.claims.exp as u64);
    if expiration.lt(current_time) {
        return Err(InvalidTime);
    }
    let not_before = Timestamp::from_seconds(token.claims.nbf as u64);
    if not_before.gt(current_time) {
        return Err(InvalidTime);
    }

    // make sure the provided hash matches the one from the tx
    let hash_bytes = general_purpose::STANDARD.decode(token.claims.transaction_hash)?;
    Ok(tx_hash.eq(&hash_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose;

    #[test]
    fn test_validate_token() {
        let encoded_token = "eyJhbGciOiJSUzI1NiIsImtpZCI6Imp3ay10ZXN0LWI5OGFjMTExLTg1MTUtNGY0OS05MDU2LTdmM2E5NzJmNzU4MSIsInR5cCI6IkpXVCJ9.eyJhdWQiOlsicHJvamVjdC10ZXN0LTE4NWU5YTlmLThiYWItNDJmMi1hOTI0LTk1M2E1OWU4ZmY5NCJdLCJleHAiOjE2OTU3NzA2MjgsImh0dHBzOi8vc3R5dGNoLmNvbS9zZXNzaW9uIjp7ImlkIjoic2Vzc2lvbi10ZXN0LTZmNDI2ZDhhLTA5M2UtNDQ1NS1hZThkLTlkMDg5MTZhMGI2NSIsInN0YXJ0ZWRfYXQiOiIyMDIzLTA5LTI2VDIzOjE4OjQ4WiIsImxhc3RfYWNjZXNzZWRfYXQiOiIyMDIzLTA5LTI2VDIzOjE4OjQ4WiIsImV4cGlyZXNfYXQiOiIyMDIzLTEwLTI2VDIzOjE4OjQ4WiIsImF0dHJpYnV0ZXMiOnsidXNlcl9hZ2VudCI6IiIsImlwX2FkZHJlc3MiOiIifSwiYXV0aGVudGljYXRpb25fZmFjdG9ycyI6W3sidHlwZSI6InBhc3N3b3JkIiwiZGVsaXZlcnlfbWV0aG9kIjoia25vd2xlZGdlIiwibGFzdF9hdXRoZW50aWNhdGVkX2F0IjoiMjAyMy0wOS0yNlQyMzoxODo0OFoifV19LCJpYXQiOjE2OTU3NzAzMjgsImlzcyI6InN0eXRjaC5jb20vcHJvamVjdC10ZXN0LTE4NWU5YTlmLThiYWItNDJmMi1hOTI0LTk1M2E1OWU4ZmY5NCIsIm5iZiI6MTY5NTc3MDMyOCwic3ViIjoidXNlci10ZXN0LTUxODUyNjQyLWE1OTQtNGE2Zi1iNzZmLTkyODUxNzI1YTQyOSIsInRyYW5zYWN0aW9uX2hhc2giOiIweDEyMzQ1Njc4OTAifQ.ToSTvPFAaFP-eIqwWSBp0z7iotclWoNkghlrecU34kAoxloEqLvooXI7Ws_-HKy1rhTWhPWOtfh4QoxObV-39pe44xPFCoN2Vv0MiutMKJSaeIC5eVHxvSz0b2jjw0WkiPj8dK8HdzscNajvMATQ9R97U_i3rluTMnvliTw0zGUUsrMfTaHcltATUJ6Ufthxvb9w2XTTsIsBx0Ttldbf0XE_ZQnFk2uNW9Skyq0-zlZZXBorEbIbbAVeA87T_4CPqp9Pdc2qXRj9XrFtXuTD1lnXk9d28tu8l4H-4CWb8DYXZrFqb9-knavNXRsKb2NJnAcTh5c_I9RvR9lVxJtkWg";
        let encoded_hash = "0x1234567890";
        let hash_bytes = general_purpose::STANDARD.decode(encoded_hash).unwrap();

        let verification = verify(
            &Timestamp::from_seconds(1695770329),
            &hash_bytes,
            encoded_token.as_bytes(),
            &"project-test-185e9a9f-8bab-42f2-a924-953a59e8ff94".to_string(),
            &"user-test-51852642-a594-4a6f-b76f-92851725a429".to_string(),
        );
        assert!(verification.unwrap());
    }
}

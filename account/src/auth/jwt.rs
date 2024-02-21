use crate::error::ContractError::{
    InvalidJWTAud, InvalidSignatureDetail, InvalidTime, InvalidToken,
};
use crate::error::ContractResult;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use cosmwasm_std::{Binary, Timestamp};
use phf::{phf_map, Map};
use rsa::traits::SignatureScheme;
use rsa::{BigUint, Pkcs1v15Sign, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str;

static AUD_KEY_MAP: Map<&'static str, &'static str> = phf_map! {
    // GA - Testnet - Test  project
    "project-test-5ae234a7-6b74-46af-a7b7-969f3df38cc0" => "4ia1pODcj-BPNblyJ1ao1etK0VltRWQEmeoQtHaCWrOES-2BCFbcOBsDDxrXPzkTUK5j15fpMFbg36vDqXiYDNPHTp7WxUrOKOSyONk4gZUd626GZwKJBryMAhU7mBMByO56sLUHdDPajykYIlpHut75gDqipDI5QY9fh_piLh7OMy-MORaWdmkv1zFqLfjAr2GUKFmd7xiUAYTsjDClTTMn1rGskjBF8qPK9jDrPz9SEwN1n7N0JPsJVRqP6m5Yf_l9JWSKarSLbV9O0qMC7Nl0MpBKTw8HTVlwaBWF-5aGbg3dMQl8Cbn4vNUv-pPjrlvrpw2m_r0Gr5N9CBEKFQ;AQAB",
    // GA - Testnet - Live project
    "project-live-7e4a3221-79cd-4f34-ac1d-fedac4bde13e" => "qm5TbnKO8tCEVdwQK1Zit0_ig2nitUzA4V_m7oePByX1oSMismJOpbgEY2xjLVCMl_JdZOUIBQvaoFx169GS0-PrKEA8sXS-20Dp8rjiEG1hSaHapRfrDPjyN5TvPPp_xNAi8YBpZ5-msK0TZmG13Rcwn9xcu74AVW0PE19s0xWGAeukoaALfgk66RdwA7_C3KKeFkaEk9VpTtVJS7e-H815L2utXaqMC7uf-Qg93l0ifVBqaJj318BdV1dBj4cliMd1k7LlSD_qmcrqYUdggJB5FquVHjSj6-j5SMBne2IzWh4GLMneS_HGoTclRCHsOGi_3BhsjgkaZt6QCLr0_fafWUinJYrnEcIjojFlWuDvzPfoSV3bRefe_IQT4-Ht8fvwVcw5wEDhBiE2lfjHjMyRG-knlM910xnEJjJjxYWbyb_fLW-NVWULFH-L91DhxlXjDwO7hbbMlGlviTcsEa3ahwszNooQ63JJdp96iSA2JgWY6JPvWHG0mNrEU3AC6UMHLUtI2Hpg1ij6tiieFUMvFLvjj7dCozpDnZr2z6msCyTgUAmO3KQHaQ3Rvo2WwyuJPzOJLBnefLZIqZzAOXHAjI_bPTTOte1vPYkfLJxLKncdd-1OCwoLMyWAdCpD4gpIsam3jPhhQfAOio1XI1BXtDMxqIyXtCQD94ycwtU;AQAB",
    // Exodvs - Test project
    "project-test-185e9a9f-8bab-42f2-a924-953a59e8ff94" => "sQKkA829tzjU2VA-INHvdrewkbQzjpsMn0PNM7KJaBODbB4ItZM4x1NVSWBiy2DGHkaDDvADRbbq1BZsC1iXVtIYm0AoD7x4QC1w89kp2_s0wmvUOSPiQZlYrgJqRDXirXJZX3MNku2McXbwdyPajDaR4nBBQOoUOF21CHqLDqBHs2R6tHyL80R_8mgueiqQ-4wg6SSVcB_6ZOh59vRcjKr34upKPWGQzvMGCkeTO9whzbIWbA1j-8ykiS63EhjWBZU_sSolsf1ZGq8peVrADDLhOvHtZxCZLKwB46k2kb8GKAWlO4wRP6BDVjzpnea7BsvZ6JwULKg3HisH9gzaiQ;AQAB",
    "integration-test-project" => "olg7TF3aai-wR4HTDe5oR-WRhEsdW3u-O3IJHl0BiHkmR4MLskHG9HzivWoXsloUBnBMrFNxOH0x5cNMI07oi4PeRbHySiogRW9CXPjJaNlTi-pT_IgKFsyJNXsLyzrnajLkDbQU6pRsHmNeL0hAOUv48rtXv8VVWWN8okJehD2q9N7LHoFAOmIUEPg_VTHTt8K__O-9eMZKN4eMjh_4-sxRX6NXPSPT87XRlrK4GZ4pUdp86K0tOFLhwO4Uj0JkMNfI82eVZ1tAbDlqjd8jFnAb8fWm8wtdaTNbL_AAXmbDhswwJOyrw8fARZIhrXSdKBWa6e4k7sLwTIy-OO8saebnlARsjGst7ZCzmw5KCm2ctEVl3hYhHwyXu_A5rOblMrV3H0G7WqeKMCMVSJ11ssrlsmfVhNIwu1Qlt5GYmPTTJiCgGUGRxZkgDyOyjFNHglYpZamCGyJ9oyofsukEGoqMQ6WzjFi_hjVapzXi7Li-Q0OjEopIUUDDgeUrgjbGY0eiHI6sAz5hoaD0Qjc9e3Hk6-y7VcKCTCAanZOlJV0vJkHB98LBLh9qAoVUei_VaLFe2IcfVlrL_43aXlsHhr_SUQY5pHPlUMbQihE_57dpPRh31qDX_w6ye8dilniP8JmpKM2uIwnJ0x7hfJ45Qa0oLHmrGlzY9wi-RGP0YUk;AQAB",
};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    aud: Box<[String]>, // Optional. Audience
    exp: u64, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: u64, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    nbf: u64, // Optional. Not Before (as UTC timestamp)
    sub: String, // Optional. Subject (whom token refers to)

    transaction_hash: Binary,
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

    // complete the time check
    //
    // timing in cosmos is unstable to say the least. therefore we have noticed
    // that the perceived time in the chain can swing quite a bit, and is almost
    // exclusively in the past. Therefore, NBF (not before) checks, which are
    // primarily set at time of JWT creation, almost always fail. Knowing this,
    // we have decided to only check expiration
    let expiration = Timestamp::from_seconds(claims.exp);
    if expiration.lt(current_time) {
        return Err(InvalidTime {
            current: current_time.seconds(),
            received: expiration.seconds(),
        });
    }
    // make sure the provided hash matches the one from the tx
    if tx_hash.eq(&claims.transaction_hash) {
        Ok(true)
    } else {
        Err(InvalidSignatureDetail {
            expected: URL_SAFE_NO_PAD.encode(tx_hash),
            received: URL_SAFE_NO_PAD.encode(claims.transaction_hash),
        })
    }
}

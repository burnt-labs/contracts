use crate::error::{ContractError::RebuildingKey, ContractResult};
use cosmwasm_std::Binary;
use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
use p256::EncodedPoint;
use serde::{Deserialize, Serialize};

pub fn verify(tx_hash: &[u8], sig_bytes: &[u8], pubkey_bytes: &Binary) -> ContractResult<bool> {
    let encoded_point = match EncodedPoint::from_bytes(pubkey_bytes) {
        Ok(point) => point,
        Err(_) => return Err(RebuildingKey),
    };
    let verifying_key: VerifyingKey = VerifyingKey::from_encoded_point(&encoded_point)?;

    let signature: Signature = Signature::from_bytes(sig_bytes.into())?;
    verifying_key.verify(tx_hash, &signature)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use crate::auth::secp256r1::verify;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use base64::Engine as _;
    use cosmwasm_std::Binary;
    use p256::ecdsa::{
        signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey,
    };
    use p256::PublicKey;
    use sha2::{Digest, Sha256};
    use webauthn_rs::prelude::*;

    #[test]
    fn test_verify_signature() {
        let key_serialized = "3ee21644150adb50dc4c20e330184fabf12e75ecbf31fe167885587e6ebf2255";
        let key_bytes = hex::decode(key_serialized).unwrap();
        let signing_key = SigningKey::from_bytes(key_bytes.as_slice().into()).unwrap();
        println!("signing key: {}", hex::encode(signing_key.to_bytes()));

        let test_value = "test_value".as_bytes();
        let signature: Signature = signing_key.sign(test_value);
        let signature_bytes = signature.to_bytes();
        println!("signature: {}", hex::encode(signature_bytes));

        let verifying_key = VerifyingKey::from(&signing_key);
        let verifying_key_bytes = verifying_key.to_encoded_point(true);
        println!("verifying key: {}", hex::encode(verifying_key_bytes));

        assert_eq!(
            true,
            verify(
                &test_value.to_vec(),
                signature_bytes.as_slice(),
                &verifying_key_bytes.as_bytes().into(),
            )
            .unwrap()
        );

        // test with invalid msg
        let bad_value = "invalid starting msg".as_bytes();
        let result = verify(
            &bad_value.to_vec(),
            signature_bytes.as_slice(),
            &verifying_key_bytes.as_bytes().into(),
        );
        assert!(result.is_err())
    }

    #[test]
    fn test_passkey_example() {
        let attestationObject64 = "o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YViksGMBiDcEppiMfxQ10TPCe2+FaKrLeTkvpzxczngTMw1BAAAAAK3OAAI1vMYKZIsLJfHwVQMAIIXSiBr3oleDaOiKWdVMJ179ljBLR8d61bocAOLV7v4ApQECAyYgASFYICgIYcuFbYyEcbB2iETp7QvVj5xSnPs+arpoIS4dPbVTIlgg5ZyECtd77CwU6JCsD71UzNTjKDi+ZY3rRFw/iOeLyo4=";
        let attestationObjectBytes = URL_SAFE_NO_PAD.decode(attestationObject64).unwrap();
        let clientData64 = "eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIiwiY2hhbGxlbmdlIjoiZEdWemRDMWphR0ZzYkdWdVoyVSIsIm9yaWdpbiI6Imh0dHBzOi8veGlvbi1kYXBwLWV4YW1wbGUtZ2l0LWZlYXQtZmFjZWlkLWJ1cm50ZmluYW5jZS52ZXJjZWwuYXBwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlLCJvdGhlcl9rZXlzX2Nhbl9iZV9hZGRlZF9oZXJlIjoiZG8gbm90IGNvbXBhcmUgY2xpZW50RGF0YUpTT04gYWdhaW5zdCBhIHRlbXBsYXRlLiBTZWUgaHR0cHM6Ly9nb28uZ2wveWFiUGV4In0=";
        let id = "hdKIGveiV4No6IpZ1UwnXv2WMEtHx3rVuhwA4tXu_gA";
        let challenge = "test-challenge";

        let rp_id = "vercel.app";
        let rp_origin =
            Url::parse("https://xion-dapp-example-git-feat-faceid-burntfinance.vercel.app")
                .expect("Invalid URL");
        let mut builder = WebauthnBuilder::new(rp_id, &rp_origin).expect("Invalid configuration");
        let webauthn = builder.build().expect("Invalid configuration");

        let register_credential: RegisterPublicKeyCredential = serde_json::from_str(r#"{"type":"public-key","id":"tqoou7foSdzMMDEwlnlPs5b6zcOnTpPxd4DxzI-0JGI","rawId":"tqoou7foSdzMMDEwlnlPs5b6zcOnTpPxd4DxzI-0JGI","authenticatorAttachment":"platform","response":{"clientDataJSON":"eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIiwiY2hhbGxlbmdlIjoiZEdWemRDMWphR0ZzYkdWdVoyVSIsIm9yaWdpbiI6Imh0dHBzOi8veGlvbi1kYXBwLWV4YW1wbGUtZ2l0LWZlYXQtZmFjZWlkLWJ1cm50ZmluYW5jZS52ZXJjZWwuYXBwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ","attestationObject":"o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YViksGMBiDcEppiMfxQ10TPCe2-FaKrLeTkvpzxczngTMw1BAAAAAK3OAAI1vMYKZIsLJfHwVQMAILaqKLu36EnczDAxMJZ5T7OW-s3Dp06T8XeA8cyPtCRipQECAyYgASFYIIxx2LdI3eZ01RVc4CxZSNbp4ifEsccV1A40sV4wP-7BIlggCfb5XwQ_CBT6BEzKJKiHDrdcAJDMjrMCIqvBjHl119c","transports":["internal"]},"clientExtensionResults":{}}"#).unwrap();

        let passkey = webauthn
            .finish_passkey_registration(&register_credential)
            .unwrap();
        // let pubkey_string = "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEPPpfp8DEYylII6VCFy_6A8j_NvcJlixhvWWnJxzuW1YnuO3p-wMRJZO_u_iNxLeUJIrrZvaN24WNvsH0VWtm9g";
        // let client_data_json = "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiZEdWemRHbHVadyIsIm9yaWdpbiI6Imh0dHA6Ly9sb2NhbGhvc3Q6ODAwMCIsImNyb3NzT3JpZ2luIjpmYWxzZX0";
        // let sig_string = "MEYCIQDvd4bmo9c8q2hXY30WHcXRywCOfnsFbhciaUxR5UQg6QIhAJ8qQA1YxJNhnlP6wZbFjFb990oPoNrBQIxxN3Rd_lb9";
        //
        // let pubkey_bytes = URL_SAFE_NO_PAD.decode(pubkey_string).unwrap();
        // println!("encoded: {}", STANDARD.encode(pubkey_bytes.clone()));
        // let sig_bytes = URL_SAFE_NO_PAD.decode(sig_string).unwrap();
        // let client_data = URL_SAFE_NO_PAD.decode(client_data_json).unwrap();
        // let mut hasher = Sha256::new();
        // hasher.update(client_data);
        // let digest = hasher.finalize().as_slice().to_vec();
        //
        // let public_key: PublicKey = cosmwasm_std::from_slice(pubkey_bytes.as_slice()).unwrap();
        // let verifying_key = VerifyingKey::from(&public_key);
        //
        // let signature: Signature = Signature::from_bytes(sig_bytes.as_slice().into()).unwrap();
        // let result = verifying_key.verify(digest.as_slice(), &signature).unwrap();
    }
}

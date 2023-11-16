use crate::error::ContractError::InvalidToken;
use crate::error::ContractResult;
use coset::iana::Algorithm;
use cosmwasm_std::{from_binary, Addr, Binary, Deps};
use futures::executor::block_on;
use passkey::authenticator::{Authenticator, UserValidationMethod};
use passkey::client::Client;
use passkey::types::ctap2::Aaguid;
use passkey::types::webauthn::*;
use passkey::types::Passkey;
use url::Url;

struct MyUserValidationMethod {}
#[async_trait::async_trait]
impl UserValidationMethod for MyUserValidationMethod {
    async fn check_user_verification(&self) -> bool {
        true
    }

    async fn check_user_presence(&self) -> bool {
        true
    }

    fn is_presence_enabled(&self) -> bool {
        true
    }

    fn is_verification_enabled(&self) -> Option<bool> {
        Some(true)
    }
}

pub fn register(
    contract: &Addr,
    origin: &Url,
    cred: &Binary,
    challenge: Vec<u8>,
) -> ContractResult<Passkey> {
    let credential: CreatedPublicKeyCredential = from_binary(cred)?;

    Err(InvalidToken)
}

#[cfg(test)]
mod tests {
    use crate::auth::passkey::{register, verify};
    use cosmwasm_std::to_binary;
    use passkey::types::webauthn::PublicKeyCredentialUserEntity;
    use url::Url;

    #[test]
    fn test_passkey_example() {
        let challenge = "test-challenge";

        let rp_origin =
            Url::parse("https://xion-dapp-example-git-feat-faceid-burntfinance.vercel.app")
                .expect("Invalid URL");
        let register_credential: PublicKeyCredentialUserEntity = serde_json::from_str(r#"{"type":"public-key","id":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","rawId":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","authenticatorAttachment":"platform","response":{"clientDataJSON":"eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIiwiY2hhbGxlbmdlIjoiZEdWemRDMWphR0ZzYkdWdVoyVSIsIm9yaWdpbiI6Imh0dHBzOi8veGlvbi1kYXBwLWV4YW1wbGUtZ2l0LWZlYXQtZmFjZWlkLWJ1cm50ZmluYW5jZS52ZXJjZWwuYXBwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ","attestationObject":"o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YViksGMBiDcEppiMfxQ10TPCe2-FaKrLeTkvpzxczngTMw1BAAAAAK3OAAI1vMYKZIsLJfHwVQMAIOgZ6Uh5SF8Dp3R4cXz8OJd0spbqZ2SL01T_Vaf2it-MpQECAyYgASFYINnBKEMfG6wkb9W1grSXgNAQ8lx6H7j6EcMyTSbZ91-XIlggdk2OOxV_bISxCsqFac6ZE8-gEurV4xQd7kFFYdfMqtE","transports":["internal"]},"clientExtensionResults":{}}"#).unwrap();

        let reg_bytes = to_binary(&register_credential).unwrap();
        let passkey = register(
            rp_origin.to_string(),
            &reg_bytes,
            challenge.as_bytes().to_vec(),
        )
        .unwrap();
        let passkey_bytes = to_binary(&passkey).unwrap();

        let authenticate_credential: PublicKeyCredential = serde_json::from_str(r#"{"type":"public-key","id":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","rawId":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","authenticatorAttachment":"platform","response":{"clientDataJSON":"eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiZEdWemRDMWphR0ZzYkdWdVoyVSIsIm9yaWdpbiI6Imh0dHBzOi8veGlvbi1kYXBwLWV4YW1wbGUtZ2l0LWZlYXQtZmFjZWlkLWJ1cm50ZmluYW5jZS52ZXJjZWwuYXBwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ","authenticatorData":"sGMBiDcEppiMfxQ10TPCe2-FaKrLeTkvpzxczngTMw0BAAAAAA","signature":"MEQCIF1Fm_XjFV5FjBRYXNN1WcDm0V4xbPn3sQ85gC34_FGmAiBzLYGsat3HwDcn4jh50gTW4mgGcmYqkvT2g1bfdFxElA","userHandle":null},"clientExtensionResults":{}}"#).unwrap();
        let authenticate_credential_bytes = to_binary(&authenticate_credential).unwrap();

        verify(
            rp_origin.to_string(),
            &passkey_bytes,
            &authenticate_credential_bytes,
            challenge.as_bytes().to_vec(),
        )
        .unwrap();
    }
}

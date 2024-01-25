use crate::error::ContractResult;
use crate::proto::{self, QueryWebAuthNVerifyRegisterRequest, QueryWebAuthNVerifyRegisterResponse};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::QueryRequest::{Custom, Stargate};
use cosmwasm_std::{to_binary, Addr, Binary, Deps};

#[cw_serde]
struct QueryRegisterRequest {
    addr: String,
    challenge: String,
    rp: String,
    data: Binary,
}

#[cw_serde]
struct QueryRegisterResponse {
    credential: Binary,
}

pub fn register(deps: Deps, addr: Addr, rp: String, data: Binary) -> ContractResult<Binary> {
    // let query = QueryRegisterRequest {
    //     addr: addr.clone().into(),
    //     challenge: addr.to_string(),
    //     rp,
    //     data,
    // };
    // let query_bz = to_binary(&query)?;
    //
    // let query_msg = proto::QueryWebAuthNVerifyRegisterRequest {
    //     addr: addr.into(),
    //     challenge: addr.to_string(),
    //     rp,
    //     data: data.into(),
    // };
    // let query_response = deps
    //     .querier
    //     .query::<QueryWebAuthNVerifyRegisterResponse>(&Custom::<
    //         QueryWebAuthNVerifyRegisterRequest,
    //     >(query_msg))?;
    //
    // Ok(query_response.credential.into())
    Ok(Binary::from_base64("")?)
}

#[cw_serde]
struct QueryVerifyRequest {
    addr: String,
    challenge: String,
    rp: String,
    credential: Binary,
    data: Binary,
}

pub fn verify(
    deps: Deps,
    addr: Addr,
    rp: String,
    signature: &Binary,
    tx_hash: Vec<u8>,
    credential: &Binary,
) -> ContractResult<bool> {
    let challenge = URL_SAFE_NO_PAD.encode(tx_hash);

    let query = QueryVerifyRequest {
        addr: addr.into(),
        challenge,
        rp,
        credential: credential.clone(),
        data: signature.clone(),
    };
    let query_bz = to_binary(&query)?;

    deps.querier.query(&Stargate {
        path: "xion.v1.Query/WebAuthNVerifyAuthenticate".to_string(),
        data: query_bz,
    })?;

    Ok(true)
}

// use crate::error::{ContractError, ContractResult};
// use cosmwasm_std::{from_binary, Binary};
// use webauthn_rs::prelude::{Passkey, PasskeyAuthentication, PasskeyRegistration, Url};
// // use webauthn_rs::prelude::*;
// use crate::error::ContractError::InvalidToken;
// use webauthn_rs::WebauthnBuilder;
// use webauthn_rs_core::interface::{AuthenticationState, RegistrationState};
// use webauthn_rs_proto::{COSEAlgorithm, PublicKeyCredential, UserVerificationPolicy};
//
//
//
// pub fn register(url: String, cred: &Binary, challenge: Vec<u8>) -> ContractResult<Passkey> {
//     let rp_origin = match Url::parse(&url) {
//         Ok(u) => u,
//         Err(_) => return Err(ContractError::URLParse { url }),
//     };
//
//     let reg = from_binary(cred)?;
//
//     let rp_id = rp_origin.domain().ok_or(ContractError::URLParse { url })?;
//     let builder = WebauthnBuilder::new(rp_id, &rp_origin)?;
//     let webauthn = builder.build()?;
//
//     let registration_state = RegistrationState {
//         policy: UserVerificationPolicy::Preferred,
//         exclude_credentials: vec![],
//         challenge: challenge.into(),
//         credential_algorithms: vec![COSEAlgorithm::ES256],
//         require_resident_key: false,
//         authenticator_attachment: None,
//         extensions: Default::default(),
//         experimental_allow_passkeys: true,
//     };
//
//     let passkey = webauthn.finish_passkey_registration(
//         &reg,
//         &PasskeyRegistration {
//             rs: registration_state,
//         },
//     )?;
//
//     Ok(passkey)
// }
//
// pub fn verify(
//     url: String,
//     passkey_bytes: &Binary,
//     cred: &Binary,
//     tx_bytes: Vec<u8>,
// ) -> ContractResult<()> {
//     let rp_origin = match Url::parse(&url) {
//         Ok(u) => u,
//         Err(_err) => return Err(ContractError::URLParse { url }),
//     };
//
//     let rp_id = rp_origin.domain().ok_or(ContractError::URLParse { url })?;
//     let builder = WebauthnBuilder::new(rp_id, &rp_origin).expect("Invalid configuration");
//     let webauthn = builder.build().expect("Invalid configuration");
//
//     let passkey: Passkey = from_binary(passkey_bytes)?;
//
//     let authentication_state = AuthenticationState {
//         credentials: vec![passkey.into()],
//         policy: UserVerificationPolicy::Preferred,
//         challenge: tx_bytes.into(),
//         appid: None,
//         allow_backup_eligible_upgrade: false,
//     };
//
//     let public_key_credential: PublicKeyCredential = from_binary(cred)?;
//
//     webauthn.finish_passkey_authentication(
//         &public_key_credential,
//         &PasskeyAuthentication {
//             ast: authentication_state,
//         },
//     )?;
//
//     Ok(())
// }
//
// #[cfg(test)]
// mod tests {
//     use crate::auth::passkey::{register, verify};
//     use cosmwasm_std::to_binary;
//     use webauthn_rs::prelude::*;
//
//     #[test]
//     fn test_passkey_example() {
//         let challenge = "test-challenge";
//
//         let rp_origin =
//             Url::parse("https://xion-dapp-example-git-feat-faceid-burntfinance.vercel.app")
//                 .expect("Invalid URL");
//         let register_credential: RegisterPublicKeyCredential = serde_json::from_str(r#"{"type":"public-key","id":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","rawId":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","authenticatorAttachment":"platform","response":{"clientDataJSON":"eyJ0eXBlIjoid2ViYXV0aG4uY3JlYXRlIiwiY2hhbGxlbmdlIjoiZEdWemRDMWphR0ZzYkdWdVoyVSIsIm9yaWdpbiI6Imh0dHBzOi8veGlvbi1kYXBwLWV4YW1wbGUtZ2l0LWZlYXQtZmFjZWlkLWJ1cm50ZmluYW5jZS52ZXJjZWwuYXBwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ","attestationObject":"o2NmbXRkbm9uZWdhdHRTdG10oGhhdXRoRGF0YViksGMBiDcEppiMfxQ10TPCe2-FaKrLeTkvpzxczngTMw1BAAAAAK3OAAI1vMYKZIsLJfHwVQMAIOgZ6Uh5SF8Dp3R4cXz8OJd0spbqZ2SL01T_Vaf2it-MpQECAyYgASFYINnBKEMfG6wkb9W1grSXgNAQ8lx6H7j6EcMyTSbZ91-XIlggdk2OOxV_bISxCsqFac6ZE8-gEurV4xQd7kFFYdfMqtE","transports":["internal"]},"clientExtensionResults":{}}"#).unwrap();
//
//         let reg_bytes = to_binary(&register_credential).unwrap();
//         let passkey = register(
//             rp_origin.to_string(),
//             &reg_bytes,
//             challenge.as_bytes().to_vec(),
//         )
//         .unwrap();
//         let passkey_bytes = to_binary(&passkey).unwrap();
//
//         let authenticate_credential: PublicKeyCredential = serde_json::from_str(r#"{"type":"public-key","id":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","rawId":"6BnpSHlIXwOndHhxfPw4l3SylupnZIvTVP9Vp_aK34w","authenticatorAttachment":"platform","response":{"clientDataJSON":"eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiZEdWemRDMWphR0ZzYkdWdVoyVSIsIm9yaWdpbiI6Imh0dHBzOi8veGlvbi1kYXBwLWV4YW1wbGUtZ2l0LWZlYXQtZmFjZWlkLWJ1cm50ZmluYW5jZS52ZXJjZWwuYXBwIiwiY3Jvc3NPcmlnaW4iOmZhbHNlfQ","authenticatorData":"sGMBiDcEppiMfxQ10TPCe2-FaKrLeTkvpzxczngTMw0BAAAAAA","signature":"MEQCIF1Fm_XjFV5FjBRYXNN1WcDm0V4xbPn3sQ85gC34_FGmAiBzLYGsat3HwDcn4jh50gTW4mgGcmYqkvT2g1bfdFxElA","userHandle":null},"clientExtensionResults":{}}"#).unwrap();
//         let authenticate_credential_bytes = to_binary(&authenticate_credential).unwrap();
//
//         verify(
//             rp_origin.to_string(),
//             &passkey_bytes,
//             &authenticate_credential_bytes,
//             challenge.as_bytes().to_vec(),
//         )
//         .unwrap();
//     }
// }

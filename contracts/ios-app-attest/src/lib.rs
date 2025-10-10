#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
pub mod msg;
mod query;

// the random function must be disabled in cosmwasm
#[cfg(not(feature = "library"))]
use core::num::NonZeroU32;
#[cfg(not(feature = "library"))]
use getrandom::Error;
#[cfg(not(feature = "library"))]
pub fn always_fail(_buf: &mut [u8]) -> Result<(), Error> {
    let code = NonZeroU32::new(Error::CUSTOM_START).unwrap();
    Err(Error::from(code))
}
#[cfg(not(feature = "library"))]
use getrandom::register_custom_getrandom;
#[cfg(not(feature = "library"))]
register_custom_getrandom!(always_fail);

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Binary, BlockInfo, Response, Timestamp};
    use cw_orch::core::CwEnvError;
    use super::*;
    use cw_orch::{interface};
    use cw_orch::prelude::*;
    use crate::msg::{InstantiateMsg, QueryMsg, VerifyAttestation};
    use crate::contract;
    use base64::prelude::*;

    #[interface(InstantiateMsg, Empty, QueryMsg, Empty)]
    pub struct IOSAppAttestVerifier;

    impl <Chain> Uploadable for IOSAppAttestVerifier<Chain> {
        fn wrapper() -> Box<dyn MockContract<Empty>> {
            Box::new(
                ContractWrapper::new_with_empty(
                    contract::execute,
                    contract::instantiate,
                    contract::query,
                )
            )
        }
    }

    #[test]
    fn test_verification() {
        let sender = Addr::unchecked("sender");
        // Create a new mock chain (backed by cw-multi-test)
        let chain = Mock::new_with_chain_id(&sender, "xion_testnet");
        chain.app.borrow_mut().set_block(BlockInfo{
            height: 12345,
            time: Timestamp::from_seconds(1759774870),
            chain_id: "xion_testnet".to_string(),
        });

        let app_attest_verifier_base: IOSAppAttestVerifier<Mock> = IOSAppAttestVerifier::new("ios_app_attest", chain);
        app_attest_verifier_base.upload().unwrap();

        let app_attest_init_msg = InstantiateMsg{};
        app_attest_verifier_base.instantiate(&app_attest_init_msg, None, &[]).unwrap();

        let app_id = "85A34A7PB2.com.burnt.integrityexample";
        let challenge_str = r#"{"timestamp":1759774575574,"latitude":40.40437277350917,"longitude":-74.35697807465439,"accuracy":8.772937993014178}"#;
        let key_id_str = "u08Jak74gUaNG6vqbXH697cDZkjW2Z9OQOBDzUIsVIo=";
        let cbor_str = "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";
        let challenge_b64 = BASE64_STANDARD.encode(challenge_str.as_bytes());

        let body = VerifyAttestation {
            app_id: app_id.to_string(),
            key_id: key_id_str.to_string(),
            challenge: Binary::from_base64(challenge_b64.as_str()).unwrap(),
            cbor_data: Binary::from_base64(cbor_str).unwrap(),
            dev_env: Some(true)
        };

        let query_msg = QueryMsg::VerifyAttestation(body);

        let verification_response: Result<bool, CwEnvError> = app_attest_verifier_base.query(&query_msg);
        assert!(verification_response.is_ok());
    }
}
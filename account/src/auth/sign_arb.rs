use crate::auth::util;
use crate::auth::util::{derive_addr, sha256};
use crate::error::ContractResult;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api};
use std::fmt::Binary;

#[cw_serde]
pub struct TxInfo {
    pub address: String,
    pub chain_id: String,
    pub account_number: u64,
    pub sequence: u64,
    pub pub_key: String, // todo: sort this out
}

pub fn verify(
    api: &dyn Api,
    msg_bytes: &[u8],
    sig_bytes: &[u8],
    pubkey: &[u8],
) -> ContractResult<bool> {
    let signer_s = derive_addr(util::CHAIN_BECH_PREFIX, pubkey)?;
    let signer = api.addr_validate(signer_s.as_str())?;

    let envelope_hash = wrap_message(msg_bytes, signer);

    let verification = api.secp256k1_verify(envelope_hash.as_slice(), sig_bytes, pubkey)?;
    Ok(verification)
}

fn wrap_message(msg_bytes: &[u8], signer: Addr) -> Vec<u8> {
    let msg_b64 = base64::encode(msg_bytes);

    let envelope = serde_json::json!({
          "type": "cosmos-sdk/StdTx",
    "value": {
      "msg": [
        {
          "type": "sign/MsgSignData",
          "value": {
            "signer": signer.to_string(),
            "data": msg_b64,
          }
        }
      ],
      "fee": {
        "amount": [],
        "gas": "0"
      },
      "memo": ""
    }
      });

    return sha256(envelope.to_string().as_bytes());
}

#[cfg(test)]
mod tests {
    use crate::auth::util;
    use crate::auth::util::sha256;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Api;

    #[test]
    fn test_derive_addr() {
        let pub_key = "AxVQixKMvKkMWMgEBn5E+QjXxFLLiOUNs3EG3vvsgaGs";
        let pub_key_bytes = base64::decode(pub_key).unwrap();

        let deps = mock_dependencies();
        let addr = util::derive_addr("osmo", pub_key_bytes.as_slice()).unwrap();

        let valid_addr = deps.api.addr_validate(addr.as_str()).unwrap();

        assert_eq!(
            "osmo1ee3y7m9kjn8xgqwryxmskv6ttnkj39z9w0fctn",
            valid_addr.as_str()
        );
    }

    #[test]
    fn test_verify_sign_arb() {
        let pubkey = "AxVQixKMvKkMWMgEBn5E+QjXxFLLiOUNs3EG3vvsgaGs";
        let pubkey_bytes = base64::decode(pubkey).unwrap();

        let deps = mock_dependencies();
        let signer_s = util::derive_addr("xion", pubkey_bytes.as_slice()).unwrap();
        let signer = deps.api.addr_validate(signer_s.as_str()).unwrap();

        assert_eq!(
            "xion1ee3y7m9kjn8xgqwryxmskv6ttnkj39z9yaq2t2",
            signer.as_str()
        );

        let test_msg = "WooHoo";

        let test_msg_b64 = base64::encode(test_msg);
        assert_eq!("V29vSG9v", test_msg_b64);

        let envelope = serde_json::json!({
              "type": "cosmos-sdk/StdTx",
        "value": {
          "msg": [
            {
              "type": "sign/MsgSignData",
              "value": {
                "signer": signer.to_string(),
                "data": test_msg_b64,
              }
            }
          ],
          "fee": {
            "amount": [],
            "gas": "0"
          },
          "memo": ""
        }
          });
        println!("envelope: {}", envelope.to_string());
        let env_hash = sha256(envelope.to_string().as_bytes());

        let expected_signature = "E5AKzlomNEYUjtYbdC8Boqlg2UIcHUL3tOq1e9CEcmlBMnONpPaAFQIZzJLIT6Jx87ViSTW58LJwGdFQqh0otA==";
        let expected_sig_bytes = base64::decode(expected_signature).unwrap();
        let verification = deps
            .api
            .secp256k1_verify(
                env_hash.as_slice(),
                expected_sig_bytes.as_slice(),
                pubkey_bytes.as_slice(),
            )
            .unwrap();
        assert!(verification)
    }
}

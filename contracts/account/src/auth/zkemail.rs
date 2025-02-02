use crate::error::ContractResult;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, Addr, Binary, Deps};

#[cw_serde]
pub struct ZKEmailSignature {
    proof: zkemail::ark_verifier::SnarkJsProof,
    dkim_hash: Binary,
}

pub fn verify(
    deps: Deps,
    verification_contract: &Addr,
    tx_bytes: &Binary,
    sig_bytes: &Binary,
    email_hash: &Binary,
    dkim_domain: &str,
) -> ContractResult<bool> {
    let sig: ZKEmailSignature = from_json(sig_bytes)?;

    let verification_request = zkemail::msg::QueryMsg::Verify {
        proof: Box::new(sig.proof),
        dkim_domain: dkim_domain.to_owned(),
        tx_bytes: tx_bytes.clone(),
        email_hash: email_hash.clone(),
        dkim_hash: sig.dkim_hash,
    };

    let verification_response: Binary = deps
        .querier
        .query_wasm_smart(verification_contract, &verification_request)?;

    let verified: bool = from_json(verification_response)?;

    Ok(verified)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{testing::MockApi, Uint256};

    use super::*;

    #[test]
    fn verifying_zkemail_signature() {
        let api = MockApi::default();

        // let

        // // example taken from ethers-rs:
        // // https://github.com/gakonst/ethers-rs/tree/master/ethers-signers#examples
        // let message = "hello world";
        // let address = "0x63F9725f107358c9115BC9d86c72dD5823E9B1E6";
        //
        // let r = Uint256::from_str(
        //     "49684349367057865656909429001867135922228948097036637749682965078859417767352",
        // )
        //     .unwrap();
        // let s = Uint256::from_str(
        //     "26715700564957864553985478426289223220394026033170102795835907481710471636815",
        // )
        //     .unwrap();
        // let v = 28u8;
        //
        // let mut sig = vec![];
        // sig.extend(r.to_be_bytes());
        // sig.extend(s.to_be_bytes());
        // sig.push(v);
        // assert_eq!(sig.len(), 65);
        //
        // let address_bytes = hex::decode(&address[2..]).unwrap();
        // let res = verify(&api, message.as_bytes(), &sig, &address_bytes);
        // assert!(res.is_ok());
        //
        // // let's try an invalid case
        // // we simply change the address to a different one
        // let wrong_address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        //
        // let address_bytes = hex::decode(&wrong_address[2..]).unwrap();
        // let res = verify(&api, message.as_bytes(), &sig, &address_bytes);
        // assert!(res.is_err());
    }
}

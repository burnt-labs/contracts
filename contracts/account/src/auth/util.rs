use crate::error::ContractError;
use bech32::{ToBase32, Variant};
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};
use base64::{Engine as _, engine::general_purpose};

pub fn sha256(msg: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(msg);
    hasher.finalize().to_vec()
}

pub fn base64url_encode(msg: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(msg)
}

fn ripemd160(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = Ripemd160::new();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}

pub const CHAIN_BECH_PREFIX: &str = "xion";
pub fn derive_addr(prefix: &str, pubkey_bytes: &[u8]) -> Result<String, ContractError> {
    let address_bytes = ripemd160(&sha256(pubkey_bytes));
    let address_str = bech32::encode(prefix, address_bytes.to_base32(), Variant::Bech32);

    match address_str {
        Ok(s) => Ok(s),
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64url_encode() {
        // Test empty input
        assert_eq!(base64url_encode(b""), "");
        
        // Test simple string
        assert_eq!(base64url_encode(b"hello"), "aGVsbG8");
        
        // Test with special characters
        assert_eq!(base64url_encode(b"hello world!"), "aGVsbG8gd29ybGQh");
        
        // Test binary data
        let binary_data = [0x00, 0x01, 0x02, 0x03, 0xFF];
        assert_eq!(base64url_encode(&binary_data), "AAECA_8");
        
        // Test longer binary data
        let longer_binary = [0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64];
        assert_eq!(base64url_encode(&longer_binary), "SGVsbG8gV29ybGQ");
        
        // Test that it produces URL-safe output (no + or / characters, no padding)
        let test_data = b"test data with special chars: +/=";
        let encoded = base64url_encode(test_data);
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        assert!(!encoded.ends_with('='));
    }
}

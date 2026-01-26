use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;

#[cw_serde]
pub struct VerificationState {
    pub shuffle_verifications: u64,
    pub decrypt_verifications: u64,
}

impl VerificationState {
    pub fn new() -> Self {
        Self {
            shuffle_verifications: 0,
            decrypt_verifications: 0,
        }
    }
}

pub const VERIFICATION_STATE: Item<VerificationState> = Item::new("verification_state");

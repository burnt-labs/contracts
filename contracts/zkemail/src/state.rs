use crate::ark_verifier::SnarkJsVkey;
use cw_storage_plus::Item;

pub const VKEY: Item<SnarkJsVkey> = Item::new("vkey");

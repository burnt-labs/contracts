use cw_storage_plus::Item;

/// The audience identifier registered in xion's JWK module.
/// Maps to an Audience record containing the ES256 verification key.
pub const AUD: Item<String> = Item::new("aud");

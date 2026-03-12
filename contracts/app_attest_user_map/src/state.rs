use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

#[cw_serde]
pub struct BacReading {
    pub bac_millis: u32,  // BAC * 1000 (e.g., 0.042 → 42)
    pub timestamp: u64,   // seconds since epoch
}

/// BAC elimination rate: ~0.015 per hour = 15 millis per hour
pub const BAC_ELIMINATION_RATE_MILLIS_PER_HOUR: u64 = 15;

impl BacReading {
    /// Returns the estimated current BAC in millis, accounting for
    /// metabolic elimination since the reading was taken.
    pub fn current_bac_millis(&self, now_secs: u64) -> u32 {
        let elapsed_secs = now_secs.saturating_sub(self.timestamp);
        let eliminated = (elapsed_secs * BAC_ELIMINATION_RATE_MILLIS_PER_HOUR) / 3600;
        self.bac_millis.saturating_sub(eliminated as u32)
    }

    /// Returns the timestamp (seconds) at which this reading will reach zero.
    pub fn expiry_secs(&self) -> u64 {
        self.timestamp + (self.bac_millis as u64) * 240
    }
}

#[cw_serde]
pub struct BacResponse {
    pub bac_millis: u32,          // original reading
    pub current_bac_millis: u32,  // estimated current value
    pub timestamp: u64,           // when reading was taken
}

pub struct BacReadingIndexes<'a> {
    pub expiry: MultiIndex<'a, u64, BacReading, Addr>,
}

impl IndexList<BacReading> for BacReadingIndexes<'_> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<BacReading>> + '_> {
        let v: Vec<&dyn Index<BacReading>> = vec![&self.expiry];
        Box::new(v.into_iter())
    }
}

pub fn user_map<'a>() -> IndexedMap<&'a Addr, BacReading, BacReadingIndexes<'a>> {
    let indexes = BacReadingIndexes {
        expiry: MultiIndex::new(
            |_pk: &[u8], r: &BacReading| r.expiry_secs(),
            "user_map",
            "user_map__expiry",
        ),
    };
    IndexedMap::new("user_map", indexes)
}

pub const APP_ID: Item<String> = Item::new("app_id");

use cosmos_sdk_proto::Any;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Binary, StdResult};
use cw_storage_plus::{Key, KeyDeserialize, Prefixer, PrimaryKey};

impl<'a> PrimaryKey<'a> for Authorization {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = Self;
    type SuperSuffix = Self;

    fn key(&self) -> Vec<Key> {
        vec![Key::Ref(self.0.as_slice())]
    }
}

impl<'a> Prefixer<'a> for Authorization {
    fn prefix(&self) -> Vec<Key> {
        vec![Key::Ref(self.0.as_slice())]
    }
}

impl KeyDeserialize for Authorization {
    type Output = Authorization;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(Authorization::from_vec(value)?)
    }
}

impl KeyDeserialize for &Authorization {
    type Output = Authorization;

    #[inline(always)]
    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Self::Output::from_vec(value)
    }
}

#[cw_serde]
pub struct Authorization(pub Binary);

#[cw_serde]
pub struct GrantConfig {
    description: String,
    pub allowance: Option<Binary>,
}

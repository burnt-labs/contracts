use crate::auth::Authenticator;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint64};

#[cw_serde]
pub struct InstantiateMsg {
    pub id: Uint64,
    pub authenticator: Authenticator,
    pub signature: Binary,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddAuthMethod {
        id: Uint64,
        authenticator: Authenticator,
        signature: Binary,
    },
    RemoveAuthMethod {
        id: Uint64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the pubkey associated with this account.
    #[returns(Binary)]
    AuthenticatorIDs {},

    #[returns(Binary)]
    AuthenticatorByID { id: Uint64 },
}

use crate::auth::{AddAuthenticator, Authenticator};
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
        add_authenticator: AddAuthenticator,
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

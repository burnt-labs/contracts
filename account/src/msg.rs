use crate::auth::{AddAuthenticator, Authenticator};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;

#[cw_serde]
pub struct InstantiateMsg {
    pub id: u8,
    pub authenticator: Authenticator,
    pub signature: Binary,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddAuthMethod { add_authenticator: AddAuthenticator },
    RemoveAuthMethod { id: u8 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the pubkey associated with this account.
    #[returns(Binary)]
    AuthenticatorIDs {},

    #[returns(Binary)]
    AuthenticatorByID { id: u8 },
}

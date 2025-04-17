use crate::auth::{AddAuthenticator, Authenticator};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub authenticator: AddAuthenticator,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddAuthMethod { add_authenticator: AddAuthenticator },
    RemoveAuthMethod { id: u8 },
    Emit { data: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the pubkey associated with this account.
    #[returns(Vec<u8>)]
    AuthenticatorIDs {},

    #[returns(Authenticator)]
    AuthenticatorByID { id: u8 },
}

#[cw_serde]
pub struct MigrateMsg {}

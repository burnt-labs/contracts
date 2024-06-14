use crate::grant::GrantConfig;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<Addr>,
    pub type_urls: Vec<String>,
    pub grant_configs: Vec<GrantConfig>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateAdmin {
        new_admin: Addr,
    },
    UpdateGrantConfig {
        msg_type_url: String,
        grant_config: GrantConfig,
    },
    RemoveGrantConfig {
        msg_type_url: String,
    },
    DeployFeeGrant {
        authz_granter: Addr,
        authz_grantee: Addr,
        msg_type_url: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the grant config by type url
    #[returns(Binary)]
    GrantConfigByTypeURL { msg_type_url: String },

    #[returns(Binary)]
    GrantConfigTypeURLs {},
}

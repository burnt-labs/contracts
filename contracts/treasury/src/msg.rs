use crate::grant::{FeeConfig, GrantConfig};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<Addr>,
    pub type_urls: Vec<String>,
    pub grant_configs: Vec<GrantConfig>,
    pub fee_config: FeeConfig,
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
    UpdateFeeConfig {
        fee_config: FeeConfig,
    },
    DeployFeeGrant {
        authz_granter: Addr,
        authz_grantee: Addr,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Query the grant config by type url
    #[returns(Binary)]
    GrantConfigByTypeUrl { msg_type_url: String },

    #[returns(Binary)]
    GrantConfigTypeUrls {},

    #[returns(Binary)]
    FeeConfig {},

    #[returns(Binary)]
    Admin {},
}

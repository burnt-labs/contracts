use crate::grant::{FeeConfig, GrantConfig};
use crate::state::Params;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<Addr>,
    pub type_urls: Vec<String>,
    pub grant_configs: Vec<GrantConfig>,
    pub fee_config: FeeConfig,
}

#[cw_serde]
pub struct UpdateGrant {
    pub msg_type_url: String,
    pub grant_config: GrantConfig,
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
    UpdateConfigs {
        grants: Option<Vec<UpdateGrant>>,
        fee_configs: Option<Vec<FeeConfig>>,
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
    RevokeAllowance {
        grantee: Addr,
    },
    UpdateParams {
        params: Params,
    },
    Withdraw {
        coins: Vec<Coin>,
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

    #[returns(Binary)]
    Params {},
}

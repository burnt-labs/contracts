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
    pub params: Params,
}

#[cw_serde]
pub enum ExecuteMsg {
    ProposeAdmin {
        new_admin: Addr,
    },
    AcceptAdmin {},
    CancelProposedAdmin {},
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
    RevokeAllowance {
        grantee: Addr,
    },
    UpdateParams {
        params: Params,
    },
    Withdraw {
        coins: Vec<Coin>,
    },
    Migrate {
        new_code_id: u64,
        migrate_msg: Binary,
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
    PendingAdmin {},

    #[returns(Binary)]
    Params {},
}

#[cw_serde]
pub struct MigrateMsg {}

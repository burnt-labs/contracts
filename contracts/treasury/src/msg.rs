use crate::grant::{FeeConfig, FeeConfigStorage, GrantConfig, GrantConfigStorage};
use crate::state::Params;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<Addr>,
    pub type_urls: Vec<String>,
    pub grant_configs: Vec<GrantConfigStorage>,
    pub fee_config: FeeConfigStorage,
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
        grant_config: GrantConfigStorage,
    },
    RemoveGrantConfig {
        msg_type_url: String,
    },
    UpdateFeeConfig {
        fee_config: FeeConfigStorage,
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
    #[returns(GrantConfig)]
    GrantConfigByTypeUrl {
        msg_type_url: String,
        account_address: String,
    },
    /// Query the grant config by type url
    #[returns(GrantConfigStorage)]
    RawGrantConfigByTypeUrl { msg_type_url: String },

    #[returns(Vec<String>)]
    GrantConfigTypeUrls {},

    #[returns(FeeConfig)]
    FeeConfig { address: String },

    #[returns(FeeConfigStorage)]
    RawFeeConfig {},

    #[returns(Addr)]
    Admin {},

    #[returns(Addr)]
    PendingAdmin {},

    #[returns(Params)]
    Params {},
}

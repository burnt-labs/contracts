use crate::grant::Any;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};

#[cw_serde]
pub enum ExecuteMsg {
    DeployFeeGrant {
        authz_granter: Addr,
        authz_grantee: Addr,
        authorization: Any,
    },
}

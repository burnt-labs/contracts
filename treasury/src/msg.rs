use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary};
use crate::grant::{Any, Grant};

#[cw_serde]
pub enum ExecuteMsg {
    DeployFeeGrant { authz_granter: Addr, authz_grantee: Addr, msg_type_url: String, authorization: Binary},
}
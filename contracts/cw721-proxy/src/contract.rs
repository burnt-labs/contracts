use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::DefaultCw721ProxyContract;
use crate::{ContractError, CONTRACT_NAME, CONTRACT_VERSION};
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response};
pub use cw721::*;
pub use cw_ownable::{Action, Ownership, OwnershipError};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let contract = DefaultCw721ProxyContract::default();
    contract.instantiate_with_version(deps, &env, &info, msg, CONTRACT_NAME, CONTRACT_VERSION)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let contract = DefaultCw721ProxyContract::default();
    contract.execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let contract = DefaultCw721ProxyContract::default();
    contract.query(deps, env, msg)
}

#[entry_point]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let contract = DefaultCw721ProxyContract::default();
    contract.migrate(deps, env, msg, CONTRACT_NAME, CONTRACT_VERSION)
}

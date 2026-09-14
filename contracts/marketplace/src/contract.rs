use std::env;

use crate::error::ContractError;
use crate::msg::{InstantiateMsg, MigrateMsg};
use crate::state::init_auto_increment;
use crate::state::Config;
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.config.validate()?;
    let config = Config::from_str(msg.config, deps.api)?;
    config.save(deps.storage)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    init_auto_increment(deps.storage)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // This updates version metadata only. It does not backfill the collection index for listings
    // created by older contract versions, so 0.2.0 is intended for fresh deployments.
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new()
        .add_attribute("method", "migrate")
        .add_attribute("version", CONTRACT_VERSION))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cw2::{get_contract_version, set_contract_version};

    use super::{migrate, MigrateMsg, CONTRACT_NAME, CONTRACT_VERSION};

    #[test]
    fn migrate_updates_contract_version() {
        let mut deps = mock_dependencies();
        set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "0.1.0").unwrap();

        migrate(deps.as_mut(), mock_env(), MigrateMsg {}).unwrap();

        let version = get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(version.contract, CONTRACT_NAME);
        assert_eq!(version.version, CONTRACT_VERSION);
    }
}

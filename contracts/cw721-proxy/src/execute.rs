use crate::msg::{get_inner, ExecuteMsg, InnerExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::DefaultCw721ProxyContract;
use crate::ContractError;
use crate::ContractError::Unauthorized;
use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};
use cw721::traits::{Cw721Execute, Cw721Query};

impl DefaultCw721ProxyContract<'static> {
    pub fn instantiate_with_version(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        msg: InstantiateMsg,
        contract_name: &str,
        contract_version: &str,
    ) -> Result<Response<Empty>, ContractError> {
        // set  the proxy addr
        self.proxy_addr.save(deps.storage, &msg.proxy_addr)?;

        // passthrough the rest
        Ok(self.base_contract.instantiate_with_version(
            deps,
            env,
            info,
            msg.inner_msg,
            contract_name,
            contract_version,
        )?)
    }

    pub fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response<Empty>, ContractError> {
        match msg {
            ExecuteMsg::UpdateExtension { msg: proxy_msg } => {
                let proxy_addr = self.proxy_addr.load(deps.storage)?;
                if info.sender.ne(&proxy_addr) {
                    return Err(Unauthorized);
                }

                let new_info = MessageInfo {
                    sender: proxy_msg.sender,
                    funds: info.clone().funds,
                };
                Ok(self
                    .base_contract
                    .execute(deps, &env, &new_info, proxy_msg.msg)?)
            }
            _ => {
                let inner_msg: InnerExecuteMsg = get_inner(msg)?;
                Ok(self.base_contract.execute(deps, &env, &info, inner_msg)?)
            }
        }
    }

    pub fn query(&self, deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        Ok(self.base_contract.query(deps, &env, msg)?)
    }

    pub fn migrate(
        &self,
        deps: DepsMut,
        env: Env,
        msg: MigrateMsg,
        contract_name: &str,
        contract_version: &str,
    ) -> Result<Response, ContractError> {
        Ok(self
            .base_contract
            .migrate(deps, env, msg, contract_name, contract_version)?)
    }
}

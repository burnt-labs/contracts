#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmos_sdk_proto::cosmos::base::v1beta1::Coin;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, AnyMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::{ContractError, ContractResult};
use crate::msg::{AdminResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{TokenInfo, TOKEN_INFO};
use cosmos_sdk_proto::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgChangeAdmin, MsgForceTransfer, MsgMint, MsgSetDenomMetadata,
};
use cw20::{BalanceResponse, Cw20ReceiveMsg, TokenInfoResponse};
use cw20_base::allowances::{
    deduct_allowance, execute_decrease_allowance, execute_increase_allowance, query_allowance,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:tf20";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn denom(deps: Deps) -> StdResult<String> {
    let token_info = TOKEN_INFO.load(deps.storage)?;
    Ok(token_info.denom)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // This initializes a new contract which acts as a wrapper and admin of a
    // token that has previously been created in x/tokenfactory
    // The recommended flow is:
    // 1. creator creates a new token in x/tokenfactory, setting themselves as the token admin
    // 2. creator creates an instance of this contract with the relevant information set
    // 3. creator transfers token admin control in x/tokenfactory from themselves, to this contract

    // because this contract needs to be the admin of the TF denom, it acts as a
    // passthrough admin for the admin of the contract. The contract admin can
    // do all the same things the TF admin can.
    // Similar to TF denom admin, you can choose to remove the admin value and
    // have the token be admin-free, which means that tokens can no longer be
    // minted or "forced" via the admin commands.

    // store token info using cw20-base format
    let data = TokenInfo {
        denom: msg.denom,
        admin: Some(info.sender),
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // admin functions for the contract to control the tokenfactory denom
        ExecuteMsg::Mint { recipient, amount } => Ok(mint(deps, env, info, recipient, amount)?),
        ExecuteMsg::ForceTransfer {
            owner,
            recipient,
            amount,
        } => Ok(force_transfer(deps, env, info, owner, recipient, amount)?),
        ExecuteMsg::ForceBurn { owner, amount } => Ok(force_burn(deps, env, info, owner, amount)?),
        ExecuteMsg::ForceSend {
            owner,
            contract,
            amount,
            msg,
        } => Ok(force_send(deps, env, info, owner, contract, amount, msg)?),
        ExecuteMsg::UpdateContractAdmin { new_admin } => {
            Ok(update_contract_admin(deps, env, info, new_admin)?)
        }
        ExecuteMsg::UpdateTokenFactoryAdmin { new_admin } => {
            Ok(update_tokenfactory_admin(deps, env, info, new_admin)?)
        }
        ExecuteMsg::ModifyMetadata { metadata } => Ok(modify_metadata(deps, env, info, metadata)?),

        // these all come from cw20-base to implement the cw20 standard
        ExecuteMsg::Transfer { recipient, amount } => {
            Ok(transfer(deps, env, info, recipient, amount)?)
        }
        ExecuteMsg::Burn { amount } => Ok(burn(deps, env, info, amount)?),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(send(deps, env, info, contract, amount, msg)?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(transfer_from(deps, env, info, owner, recipient, amount)?),
        ExecuteMsg::BurnFrom { owner, amount } => Ok(burn_from(deps, env, info, owner, amount)?),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(send_from(deps, env, info, owner, contract, amount, msg)?),
    }
}

pub fn assert_admin(deps: Deps, sender: Addr) -> ContractResult<()> {
    // asserts that the sender is the contract admin for this instance
    // if an admin is not set, always fail
    let token_info = TOKEN_INFO.load(deps.storage)?;
    match token_info.admin {
        None => Err(ContractError::Unauthorized),
        Some(admin) => {
            if sender != admin {
                return Err(ContractError::Unauthorized);
            }
            Ok(())
        }
    }
}

pub fn mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    assert_admin(deps.as_ref(), info.sender)?;

    deps.api.addr_validate(&recipient)?;

    let denom = denom(deps.as_ref())?;
    let coin = Coin {
        denom,
        amount: amount.clone().to_string(),
    };

    let force_transfer_msg = MsgMint {
        sender: env.contract.address.into_string(),
        amount: Some(coin),
        mint_to_address: recipient.clone(),
    };
    let any_msg = AnyMsg {
        type_url: String::from("/osmosis.tokenfactory.v1beta1.Msg/Mint"),
        value: to_json_binary(&force_transfer_msg)?,
    };

    let res = Response::new()
        .add_attribute("action", "mint")
        .add_attribute("to", recipient)
        .add_attribute("amount", amount)
        .add_message(CosmosMsg::Any(any_msg));
    Ok(res)
}

pub fn force_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    assert_admin(deps.as_ref(), info.sender)?;

    deps.api.addr_validate(&owner)?;
    deps.api.addr_validate(&recipient)?;

    _transfer(deps, env, owner, recipient, amount)
}

pub fn force_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    assert_admin(deps.as_ref(), info.sender)?;

    deps.api.addr_validate(&owner)?;
    _burn(deps, env, owner, amount)
}

pub fn force_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    assert_admin(deps.as_ref(), info.sender)?;

    deps.api.addr_validate(&owner)?;
    deps.api.addr_validate(&contract)?;
    _send(deps, env, owner, contract, amount, msg)
}

pub fn update_contract_admin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    assert_admin(deps.as_ref(), info.sender.clone())?;
    let old_admin = info.sender.into_string();

    let admin = match new_admin.is_empty() {
        true => None,
        false => {
            let addr = deps.api.addr_validate(&new_admin)?;
            Some(addr)
        }
    };

    let mut token_info = TOKEN_INFO.load(deps.storage)?;
    token_info.admin = admin;
    TOKEN_INFO.save(deps.storage, &token_info)?;
    Ok(Response::new()
        .add_attribute("action", "update_contract_admin")
        .add_attribute("old_admin", old_admin)
        .add_attribute("new_admin", new_admin))
}

pub fn update_tokenfactory_admin(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    assert_admin(deps.as_ref(), info.sender.clone())?;
    let old_admin = info.sender.clone().into_string();

    let denom = denom(deps.as_ref())?;

    let change_admin_msg = MsgChangeAdmin {
        sender: info.sender.into_string(),
        denom,
        new_admin: new_admin.clone(),
    };

    let any_msg = AnyMsg {
        type_url: String::from("/osmosis.tokenfactory.v1beta1.Msg/ChangeAdmin"),
        value: to_json_binary(&change_admin_msg)?,
    };

    Ok(Response::new()
        .add_attribute("action", "update_tokenfactory_admin")
        .add_attribute("old_admin", old_admin)
        .add_attribute("new_admin", new_admin)
        .add_message(CosmosMsg::Any(any_msg)))
}

pub fn modify_metadata(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    metadata: Binary,
) -> Result<Response, ContractError> {
    assert_admin(deps.as_ref(), info.sender.clone())?;

    let deserialized_metadata = from_json(metadata)?;

    let change_admin_msg = MsgSetDenomMetadata {
        sender: info.sender.into_string(),
        metadata: Some(deserialized_metadata),
    };

    let any_msg = AnyMsg {
        type_url: String::from("/osmosis.tokenfactory.v1beta1.Msg/SetDenomMetadata"),
        value: to_json_binary(&change_admin_msg)?,
    };

    Ok(Response::new()
        .add_attribute("action", "modify_metadata")
        .add_message(CosmosMsg::Any(any_msg)))
}

pub fn transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    _transfer(deps, env, info.sender.into_string(), recipient, amount)
}

pub fn transfer_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&recipient)?;
    let owner_addr = deps.api.addr_validate(&owner)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;

    _transfer(deps, env, owner, recipient, amount)
}

pub fn _transfer(
    deps: DepsMut,
    env: Env,
    sender: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let denom = denom(deps.as_ref())?;
    let coin = Coin {
        denom,
        amount: amount.clone().to_string(),
    };

    let force_transfer_msg = MsgForceTransfer {
        sender: env.contract.address.into_string(),
        amount: Some(coin),
        transfer_from_address: sender.clone(),
        transfer_to_address: recipient.clone(),
    };
    let any_msg = AnyMsg {
        type_url: String::from("/osmosis.tokenfactory.v1beta1.Msg/ForceTransfer"),
        value: to_json_binary(&force_transfer_msg)?,
    };

    let res = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", sender)
        .add_attribute("to", recipient)
        .add_attribute("amount", amount)
        .add_message(CosmosMsg::Any(any_msg));
    Ok(res)
}

pub fn send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&contract)?;
    _send(deps, env, info.sender.into_string(), contract, amount, msg)
}

pub fn send_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&contract)?;
    let owner_addr = deps.api.addr_validate(&owner)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;

    _send(deps, env, info.sender.into_string(), contract, amount, msg)
}

pub fn _send(
    deps: DepsMut,
    env: Env,
    sender: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    deps.api.addr_validate(&contract)?;

    let denom = denom(deps.as_ref())?;
    let coin = Coin {
        denom,
        amount: amount.clone().to_string(),
    };

    let force_transfer_msg = MsgForceTransfer {
        sender: env.contract.address.into_string(),
        amount: Some(coin),
        transfer_from_address: sender.clone(),
        transfer_to_address: contract.clone(),
    };
    let any_msg = AnyMsg {
        type_url: String::from("/osmosis.tokenfactory.v1beta1.Msg/ForceTransfer"),
        value: to_json_binary(&force_transfer_msg)?,
    };

    let res = Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", &sender)
        .add_attribute("to", &contract)
        .add_attribute("amount", amount)
        .add_message(
            Cw20ReceiveMsg {
                sender,
                amount,
                msg,
            }
            .into_cosmos_msg(contract)?,
        )
        .add_message(CosmosMsg::Any(any_msg));
    Ok(res)
}

pub fn burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    _burn(deps, env, info.sender.into_string(), amount)
}

pub fn burn_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;

    _burn(deps, env, owner, amount)
}

pub fn _burn(
    deps: DepsMut,
    env: Env,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let denom = denom(deps.as_ref())?;
    let coin = Coin {
        denom,
        amount: amount.clone().to_string(),
    };

    let burn_msg = MsgBurn {
        sender: env.contract.address.into_string(),
        amount: Some(coin),
        burn_from_address: sender.clone(),
    };
    let any_msg = AnyMsg {
        type_url: String::from("/osmosis.tokenfactory.v1beta1.Msg/Burn"),
        value: to_json_binary(&burn_msg)?,
    };

    let res = Response::new()
        .add_attribute("action", "burn")
        .add_attribute("from", sender)
        .add_attribute("amount", amount)
        .add_message(CosmosMsg::Any(any_msg));
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // inherited from cw20-base
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::Allowance { owner, spender } => {
            to_json_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::Admin {} => to_json_binary(&query_admin(deps)?),
    }
}

pub fn query_token_info(deps: Deps) -> StdResult<TokenInfoResponse> {
    let denom = denom(deps)?;
    let metadata = deps.querier.query_denom_metadata(denom.clone())?;
    let supply = deps.querier.query_supply(denom)?;

    let exponent = match metadata
        .denom_units
        .iter()
        .find(|&d| d.denom == metadata.base)
    {
        None => 0,
        Some(denom_unit) => denom_unit.exponent,
    };
    let res = TokenInfoResponse {
        name: metadata.name,
        symbol: metadata.symbol,
        decimals: exponent as u8,
        total_supply: supply.amount,
    };
    Ok(res)
}

pub fn query_balance(deps: Deps, address: String) -> StdResult<BalanceResponse> {
    deps.api.addr_validate(&address)?;

    let denom = denom(deps)?;
    let coin = deps.querier.query_balance(address, denom)?;

    Ok(BalanceResponse {
        balance: coin.amount,
    })
}

pub fn query_admin(deps: Deps) -> StdResult<AdminResponse> {
    let token_info = TOKEN_INFO.load(deps.storage)?;

    Ok(AdminResponse {
        admin: token_info.admin,
    })
}
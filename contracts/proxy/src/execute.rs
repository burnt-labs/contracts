use crate::error::ContractError::{InvalidCodeID, InvalidMsgType, Unauthorized};
use crate::error::ContractResult;
use crate::msg::ProxyMsg;
use crate::state::{ADMIN, CODE_IDS};
use cosmwasm_std::{to_json_binary, Addr, Deps, DepsMut, Event, MessageInfo, Response, WasmMsg};

pub fn init(
    deps: DepsMut,
    _: MessageInfo,
    admin: Option<Addr>,
    code_ids: Vec<u64>,
) -> ContractResult<Response> {
    ADMIN.save(deps.storage, &admin)?;

    for code_id in code_ids.clone() {
        CODE_IDS.save(deps.storage, code_id, &true)?;
    }

    let admin_str: String = match admin {
        None => String::new(),
        Some(a) => a.into_string(),
    };

    let code_ids_strs: Vec<String> = code_ids.iter().map(|n| n.to_string()).collect();
    let code_ids_str = code_ids_strs.join(", ");

    Ok(Response::new().add_event(
        Event::new("create_proxy_instance")
            .add_attributes(vec![("admin", admin_str), ("code_ids", code_ids_str)]),
    ))
}

// main logic: this contract is meant to allow a single address to represent 
// multiple or dynamic other contracts. In this case, it is any contract that 
// is backed by a particular code ID. The sender sends wrapped msgs of the 
// WasmMsg::Execute type, which includes the target contract, and the msg binary.
// This proxy contract will make any necessary in-flight checks (code ID here)
// and then submit new ProxyMsg msgs to the target contract. The receiving 
// contract must understand and authenticate such msgs

pub fn proxy_msgs(
    deps: DepsMut,
    info: MessageInfo,
    msgs: Vec<WasmMsg>,
) -> ContractResult<Response> {
    let mut proxy_msgs: Vec<WasmMsg> = Vec::with_capacity(msgs.len());

    for msg in msgs {
        let (proxy_msg, contract_addr, funds) = match msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => (
                ProxyMsg {
                    sender: info.sender.clone(),
                    msg,
                },
                contract_addr,
                funds,
            ),
            _ => return Err(InvalidMsgType),
        };
        let contract_info = deps
            .querier
            .query_wasm_contract_info(contract_addr.clone())?;
        if !CODE_IDS.has(deps.storage, contract_info.code_id) {
            return Err(InvalidCodeID {
                contract: contract_addr,
                code_id: contract_info.code_id,
            });
        }

        let exec_msg = WasmMsg::Execute {
            contract_addr,
            msg: to_json_binary(&proxy_msg)?,
            funds,
        };
        proxy_msgs.push(exec_msg);
    }

    Ok(Response::new()
        .add_event(Event::new("proxied_msgs"))
        .add_messages(proxy_msgs))
}

// administration msgs

pub fn is_admin(deps: Deps, address: Addr) -> ContractResult<()> {
    let admin = ADMIN.load(deps.storage)?;
    match admin {
        None => Err(Unauthorized),
        Some(a) => {
            if a != address {
                Err(Unauthorized)
            } else {
                Ok(())
            }
        }
    }
}

pub fn update_admin(
    deps: DepsMut,
    info: MessageInfo,
    new_admin: Option<Addr>,
) -> ContractResult<Response> {
    is_admin(deps.as_ref(), info.sender.clone())?;

    ADMIN.save(deps.storage, &new_admin)?;

    let admin_str: String = match new_admin {
        None => String::new(),
        Some(a) => a.into_string(),
    };

    Ok(
        Response::new().add_event(Event::new("updated_treasury_admin").add_attributes(vec![
            ("old admin", info.sender.into_string()),
            ("new admin", admin_str),
        ])),
    )
}
pub fn add_code_ids(
    deps: DepsMut,
    info: MessageInfo,
    code_ids: Vec<u64>,
) -> ContractResult<Response> {
    is_admin(deps.as_ref(), info.sender.clone())?;

    for code_id in code_ids.clone() {
        CODE_IDS.save(deps.storage, code_id, &true)?;
    }

    let code_ids_strs: Vec<String> = code_ids.iter().map(|n| n.to_string()).collect();
    let code_ids_str = code_ids_strs.join(", ");

    Ok(
        Response::new().add_event(Event::new("updated_proxy_code_ids").add_attributes(vec![
            ("admin", info.sender.as_str()),
            ("new_code_ids", code_ids_str.as_str()),
        ])),
    )
}

pub fn remove_code_ids(
    deps: DepsMut,
    info: MessageInfo,
    code_ids: Vec<u64>,
) -> ContractResult<Response> {
    is_admin(deps.as_ref(), info.sender.clone())?;

    for code_id in code_ids.clone() {
        CODE_IDS.remove(deps.storage, code_id);
    }

    let code_ids_strs: Vec<String> = code_ids.iter().map(|n| n.to_string()).collect();
    let code_ids_str = code_ids_strs.join(", ");

    Ok(
        Response::new().add_event(Event::new("updated_proxy_code_ids").add_attributes(vec![
            ("admin", info.sender.as_str()),
            ("new_code_ids", code_ids_str.as_str()),
        ])),
    )
}

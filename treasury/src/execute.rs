use crate::error::ContractResult;
use crate::grant::Authorization;
use crate::proto::{self, QueryGrantsResponse, XionCustomQuery};
use crate::state::GRANTS;
use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response};

pub fn deploy_fee_grant(
    deps: DepsMut<XionCustomQuery>,
    env: Env,
    info: MessageInfo,
    authz_granter: Addr,
    authz_grantee: Addr,
    msg_type_url: String,
    authorization: Authorization,
) -> ContractResult<Response> {
    // check if grant exists in patterns on contract
    let grant_config = GRANTS.load(deps.storage, authorization)?;

    // check if grant exists on chain
    let query_msg = proto::QueryGrantsRequest {
        granter: Some(authz_granter.to_string()),
        grantee: Some(authz_grantee.to_string()),
        msg_type_url: Some(msg_type_url),
        pagination: None,
    };
    let grants = deps
        .querier
        .query::<QueryGrantsResponse>(&query_msg.into())?;
    let grant = grants.grants[0].clone();

    // create feegrant, if needed
    // TODO: remove comment
    // match grant_config.allowance {
    //     None => Ok(Response::new()),
    //     Some(allowance) => {
    //         let fee_grant_msg = MsgGrantAllowance{
    //             granter: env.contract.address.into_string(),
    //             grantee: authz_grantee.to_string(),
    //             allowance: allowance,
    //         };
    //         let cosmos_msg = CosmosMsg::Stargate {
    //             type_url: ""
    //         }
    //         Ok(Response::new().add_message(fee_grant_msg))
    //     }
    // }
    return Ok(Response::default());
}

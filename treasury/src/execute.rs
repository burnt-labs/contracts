use cosmos_sdk_proto::cosmos::authz::v1beta1::{QueryGrantsRequest, QueryGrantsResponse};
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::MsgGrantAllowance;
use cosmwasm_std::{Addr, Binary, CosmosMsg, DepsMut, Env, MessageInfo, QueryRequest, Response};
use prost::Message;
use crate::error::ContractResult;
use crate::grant::{Authorization};
use crate::state::GRANTS;

pub fn deploy_fee_grant(deps: DepsMut,
                        env: Env,
                        info: MessageInfo,
                        authz_granter: Addr,
                        authz_grantee: Addr,
                        msg_type_url: String,
                        authorization: Authorization) -> ContractResult<Response> {

    // check if grant exists in patterns on contract
    let grant_config = GRANTS.load(deps.storage, authorization)?;

    // check if grant exists on chain
    let query_msg = QueryGrantsRequest{
        granter: authz_granter.to_string(),
        grantee: authz_grantee.to_string(),
        msg_type_url,
        pagination: None,
    };
    let query_request = QueryRequest::Stargate {
        path: "cosmos.authz.v1beta1.Query/Grants".to_string(),
        data: Default::default() };
    let query_response = deps.querier.query(&query_request)?;
    let grants = QueryGrantsResponse::decode(query_response)?;
    let grant = grants.grants[0].clone();
    
    // create feegrant, if needed
    match grant_config.allowance {
        None => Ok(Response::new()),
        Some(allowance) => {
            let fee_grant_msg = MsgGrantAllowance{
                granter: env.contract.address.into_string(),
                grantee: authz_grantee.to_string(),
                allowance: allowance,
            };
            let cosmos_msg = CosmosMsg::Stargate {
                type_url: ""
            }
            Ok(Response::new().add_message(fee_grant_msg))
        }
    }
}
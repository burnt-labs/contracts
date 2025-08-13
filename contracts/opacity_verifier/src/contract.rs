use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Response, StdResult};
use crate::error::{ContractError, ContractResult};
use crate::msg::{QueryMsg, ExecuteMsg, InstantiateMsg};
use crate::{query, CONTRACT_NAME, CONTRACT_VERSION};
use crate::state::{ADMIN, VERIFICATION_KEY_ALLOW_LIST};

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    ADMIN.save(deps.storage, &msg.admin)?;
    for key in msg.allow_list {
        VERIFICATION_KEY_ALLOW_LIST.save(deps.storage, key, &Empty{})?;
    }
    Ok(Response::new().add_event(Event::new("create_opacity_verifier").add_attributes( vec![
        ("admin", msg.admin.into_string()),
    ])))
}


#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender != admin {
        return Err(ContractError::Unauthorized {});
    }
    match msg {
        ExecuteMsg::UpdateAdmin { admin } => {
            ADMIN.save(deps.storage, &admin)?;
        }
        ExecuteMsg::UpdateAllowList { keys  } => {
            VERIFICATION_KEY_ALLOW_LIST.clear(deps.storage);
            for key in keys {
                VERIFICATION_KEY_ALLOW_LIST.save(deps.storage, key, &Empty{})?;
            }
        }
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Verify { signature, message } => to_json_binary(
            &query::verify_query(deps.storage, deps.api, signature, message)?),
        QueryMsg::VerificationKeys {} => to_json_binary(&query::verification_keys(deps.storage)?),
        QueryMsg::Admin {} => to_json_binary(&query::admin(deps.storage)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Addr};
    use cw_orch::interface;
    use cw_orch::prelude::*;

    #[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
    pub struct OpacityVerifier;

    impl <Chain> Uploadable for OpacityVerifier<Chain> {
        fn wrapper() -> Box<dyn MockContract<Empty>> {
            Box::new(
                ContractWrapper::new_with_empty(
                    execute,
                    instantiate,
                    query,
                )
            )
        }
    }

    #[test]
    fn test_verify_proof() {
        let allowlist_keys_raw = ["322df8c3146a9891c8d63deec562db5f325f7a28","3ac1e280b6b5d8e15cf428229eccb20d9b824a53","5a29af4859ebc29ac0819c178bd293ba7f7bdfcf","9b776cbbd434d7d8f32b8cb369c37442760457b5","90cbfa246fb5bd65192aeaaa41483e311a13f109","ae16d88cd1f4ba016da8909ebc7c9c4a4fb112b8","8a4ca92581fb9b569ef8152c70a031569ee971b5","bdd5b7410abf138da1008906191188f4b5543be7","5d92cf96045bb80d869ee7bfa5d894be4782cfab","7775b5ffbcd55e7fce07672895145c5961ff828f","cf203ffb676fad5c8924ceebe91ebe3e617f01af"];
        let allowlist_keys: Vec<String> = allowlist_keys_raw.iter().map(|x| x.to_string()).collect();

        let sender = Addr::unchecked("sender");
        // Create a new mock chain (backed by cw-multi-test)
        let chain = Mock::new(&sender);

        let opacity_verifier: OpacityVerifier<Mock> = OpacityVerifier::new("opacity_verifier", chain);
        opacity_verifier.upload().unwrap();

        let verifier_init_msg = InstantiateMsg {
            admin: sender,
            allow_list: allowlist_keys,
        };

        opacity_verifier.instantiate(&verifier_init_msg, None, &[]).unwrap();
        
        let verifier_query_msg = QueryMsg::Verify {
            signature: "0x67054ee2d920f5fe11e9c34dd20257b4bb7e9549a85aef98be9f98c564838ded3d7a84864342eac1d1991abb4becb82a4cf8476d010dfc05ce973566d1fbffe91c".to_string(),
            message: r#"{"body":"{\"login\":\"mvid\",\"id\":74642,\"node_id\":\"MDQ6VXNlcjc0NjQy\",\"avatar_url\":\"https://avatars.githubusercontent.com/u/74642?v=4\",\"gravatar_id\":\"\",\"url\":\"https://api.github.com/users/mvid\",\"html_url\":\"https://github.com/mvid\",\"followers_url\":\"https://api.github.com/users/mvid/followers\",\"following_url\":\"https://api.github.com/users/mvid/following{/other_user}\",\"gists_url\":\"https://api.github.com/users/mvid/gists{/gist_id}\",\"starred_url\":\"https://api.github.com/users/mvid/starred{/owner}{/repo}\",\"subscriptions_url\":\"https://api.github.com/users/mvid/subscriptions\",\"organizations_url\":\"https://api.github.com/users/mvid/orgs\",\"repos_url\":\"https://api.github.com/users/mvid/repos\",\"events_url\":\"https://api.github.com/users/mvid/events{/privacy}\",\"received_events_url\":\"https://api.github.com/users/mvid/received_events\",\"type\":\"User\",\"user_view_type\":\"private\",\"site_admin\":false,\"name\":\"Mantas Vidutis\",\"company\":\"Turbines Consulting, LLC\",\"blog\":\"turbines.io\",\"location\":\"San Francisco, CA\",\"email\":\"mantas.a.vidutis@gmail.com\",\"hireable\":true,\"bio\":\"Software Consultant\",\"twitter_username\":null,\"notification_email\":\"mantas.a.vidutis@gmail.com\",\"public_repos\":41,\"public_gists\":4,\"followers\":44,\"following\":60,\"created_at\":\"2009-04-17T02:12:05Z\",\"updated_at\":\"2025-06-21T08:03:51Z\",\"private_gists\":24,\"total_private_repos\":6,\"owned_private_repos\":6,\"disk_usage\":38482,\"collaborators\":1,\"two_factor_authentication\":true,\"plan\":{\"name\":\"pro\",\"space\":976562499,\"collaborators\":0,\"private_repos\":9999}}","cookies":{},"headers":{"access-control-allow-origin":"*","access-control-expose-headers":"ETag, Link, Location, Retry-After, X-GitHub-OTP, X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Used, X-RateLimit-Resource, X-RateLimit-Reset, X-OAuth-Scopes, X-Accepted-OAuth-Scopes, X-Poll-Interval, X-GitHub-Media-Type, X-GitHub-SSO, X-GitHub-Request-Id, Deprecation, Sunset","cache-control":"private, max-age=60, s-maxage=60","content-length":"1497","content-security-policy":"default-src 'none'","content-type":"application/json; charset=utf-8","date":"Fri, 27 Jun 2025 17:32:15 GMT","etag":"\"a9d561910da5ada4f578f0e92f6af450dd3df9a449030bd90abbc7e6da9bc7df\"","last-modified":"Sat, 21 Jun 2025 08:03:51 GMT","referrer-policy":"origin-when-cross-origin, strict-origin-when-cross-origin","server":"github.com","strict-transport-security":"max-age=31536000; includeSubdomains; preload","vary":"Accept, Authorization, Cookie, X-GitHub-OTP,Accept-Encoding, Accept, X-Requested-With","x-accepted-oauth-scopes":"","x-content-type-options":"nosniff","x-frame-options":"deny","x-github-api-version-selected":"2022-11-28","x-github-media-type":"github.v3; format=json","x-github-request-id":"793D:54EE8:1DC7644:1E7E84C:685ED59F","x-oauth-client-id":"Ov23liqmohfBdEpL34Ii","x-oauth-scopes":"read:user, user:email","x-ratelimit-limit":"5000","x-ratelimit-remaining":"4999","x-ratelimit-reset":"1751049135","x-ratelimit-resource":"core","x-ratelimit-used":"1","x-xss-protection":"0"},"status":200,"url":"api.github.com/user","url_params":{}}"#.to_string(),
        };
        let verification_response: bool = opacity_verifier.query(&verifier_query_msg).unwrap();
        assert!(verification_response)
    }
}
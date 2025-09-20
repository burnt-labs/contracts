use cosmwasm_std::{Addr, Coin, Deps, DepsMut, Env, MessageInfo, Response};
use cw721::{
    state::NftInfo,
    traits::{Cw721CustomMsg, Cw721State},
};

use crate::{
    error::ContractError,
    state::{AssetConfig, ListingInfo},
};

pub fn list<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    price: Coin,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: Cw721CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();
    // make sure the caller is the owner of the token
    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    // check if we can list the asset
    check_can_list(deps.as_ref(), &env, info.sender.as_ref(), &nft_info)?;
    // make sure the price is greater than zero
    if price.amount.is_zero() {
        return Err(ContractError::InvalidListingPrice {
            price: price.amount.u128(),
        });
    }
    // Ensure the listing does not already exist
    let old_listing = asset_config.listings.may_load(deps.storage, &id)?;
    if old_listing.is_some() {
        return Err(ContractError::ListingAlreadyExists { id });
    }
    // Save the listing
    let listing = ListingInfo {
        id: id.clone(),
        seller: info.sender.clone(),
        price: price.clone(),
        is_frozen: false,
        nft_info,
    };
    asset_config.listings.save(deps.storage, &id, &listing)?;
    Ok(Response::default()
        .add_attribute("action", "list")
        .add_attribute("id", id)
        .add_attribute("price", price.amount.to_string())
        .add_attribute("denom", price.denom.to_string())
        .add_attribute("seller", info.sender.clone().to_string()))
}
fn delist<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
) -> Result<Response<TCustomResponseMsg>, ContractError> {
    todo!()
}
fn freeze_listing<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
) -> Result<Response<TCustomResponseMsg>, ContractError> {
    todo!()
}
fn buy<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    recipient: Option<String>,
) -> Result<Response<TCustomResponseMsg>, ContractError> {
    todo!()
}

/// returns true if the sender can list the token
/// copied from cw721 check_can_send
fn check_can_list<TNftExtension>(
    deps: Deps,
    env: &Env,
    sender: &str,
    token: &NftInfo<TNftExtension>,
) -> Result<(), ContractError>
where
    TNftExtension: Cw721State,
{
    let sender = deps.api.addr_validate(sender)?;
    // owner can send
    if token.owner == sender {
        return Ok(());
    }

    // any non-expired token approval can send
    if token
        .approvals
        .iter()
        .any(|apr| apr.spender == sender && !apr.is_expired(&env.block))
    {
        return Ok(());
    }
    // operator can send
    let asset_config = AssetConfig::<TNftExtension>::default();
    let op = asset_config
        .cw721_config
        .operators
        // has token owner approved/gave grant to sender for full control over owner's NFTs?
        .may_load(deps.storage, (&token.owner, &sender))?;

    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(ContractError::Unauthorized {})
            } else {
                Ok(())
            }
        }
        None => Err(ContractError::Unauthorized {}),
    }
}

#[test]
fn test_list() {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{Coin, Empty};

    // successful listing stores state and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-1", &nft_info)
            .unwrap();

        let price = Coin::new(100 as u128, "uxion");
        let response = list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            price.clone(),
        )
        .unwrap();

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "list".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), owner_addr.to_string()),
            ],
        );

        let stored = AssetConfig::<Empty>::default()
            .listings
            .load(deps.as_ref().storage, "token-1")
            .unwrap();
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, owner_addr);
        assert!(!stored.is_frozen);
        assert_eq!(stored.nft_info.owner, stored.seller);

        let duplicate_err = list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            Coin::new(200 as u128, "uxion"),
        )
        .unwrap_err();
        assert_eq!(
            duplicate_err,
            ContractError::ListingAlreadyExists {
                id: "token-1".to_string(),
            }
        );
    }

    // non-owner cannot list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let nft_info = NftInfo {
            owner: deps.api.addr_make("owner"),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-2", &nft_info)
            .unwrap();

        let intruder_addr = deps.api.addr_make("intruder");
        let err = list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-2".to_string(),
            Coin::new(100 as u128, "uxion"),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    // approvals can list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let approver_addr = deps.api.addr_make("approver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![cw721::Approval {
                spender: approver_addr.clone(),
                expires: cw721::Expiration::AtHeight(env.block.height + 100),
            }],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-3", &nft_info)
            .unwrap();

        let price = Coin::new(100 as u128, "uxion");
        let response = list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&approver_addr, &[]),
            "token-3".to_string(),
            price.clone(),
        )
        .unwrap();

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "list".to_string()),
                ("id".to_string(), "token-3".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), approver_addr.to_string()),
            ],
        );

        let stored = AssetConfig::<Empty>::default()
            .listings
            .load(deps.as_ref().storage, "token-3")
            .unwrap();
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, approver_addr);
        assert!(!stored.is_frozen);
        assert_eq!(stored.nft_info.owner, owner_addr);
    }

    // operators can list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let operator_addr = deps.api.addr_make("operator");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        let asset_config = AssetConfig::<Empty>::default();
        asset_config
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-4", &nft_info)
            .unwrap();
        asset_config
            .cw721_config
            .operators
            .save(
                deps.as_mut().storage,
                (&owner_addr, &operator_addr),
                &cw721::Expiration::AtHeight(env.block.height + 100),
            )
            .unwrap();

        let price = Coin::new(100 as u128, "uxion");
        let response = list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&operator_addr, &[]),
            "token-4".to_string(),
            price.clone(),
        )
        .unwrap();

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "list".to_string()),
                ("id".to_string(), "token-4".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), operator_addr.to_string()),
            ],
        );

        let stored = AssetConfig::<Empty>::default()
            .listings
            .load(deps.as_ref().storage, "token-4")
            .unwrap();
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, operator_addr);
        assert!(!stored.is_frozen);
        assert_eq!(stored.nft_info.owner, owner_addr);
    }

    // expired approvals or operators cannot list
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let exp_approval_addr = deps.api.addr_make("bad_actor");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![cw721::Approval {
                spender: exp_approval_addr.clone(),
                expires: cw721::Expiration::AtHeight(env.block.height - 1),
            }],
            token_uri: None,
            extension: Empty {},
        };
        let asset_config = AssetConfig::<Empty>::default();
        asset_config
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-5", &nft_info)
            .unwrap();
        asset_config
            .cw721_config
            .operators
            .save(
                deps.as_mut().storage,
                (&owner_addr, &exp_approval_addr),
                &cw721::Expiration::AtHeight(env.block.height - 1),
            )
            .unwrap();

        let err = list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&exp_approval_addr, &[]),
            "token-5".to_string(),
            Coin::new(100 as u128, "uxion"),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    // zero-priced listing is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-3", &nft_info)
            .unwrap();

        let err = list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-3".to_string(),
            Coin::new(0 as u128, "uxion"),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::InvalidListingPrice { price: 0 });
    }
}

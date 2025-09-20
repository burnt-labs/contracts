use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
};
use cw721::{
    state::NftInfo,
    traits::{Cw721CustomMsg, Cw721State}, Expiration,
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
        reserved_until: None,
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
pub fn delist<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: Cw721CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or_else(|| ContractError::ListingNotFound { id: id.clone() })?;

    if listing.seller != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let reserved_listing = asset_config
        .reserved_listings
        .may_load(deps.storage, &id)?;
    if let Some(reserved_listing) = reserved_listing {
        if let Some(until) = reserved_listing.reserved_until {
            if !until.is_expired(&env.block) {
                return Err(ContractError::ReservedAsset { id: id.clone() });
            } // else the reservation has expired, we can delist
            asset_config.reserved_listings.remove(deps.storage, &id)?;
        }
    }

    asset_config.listings.remove(deps.storage, &id)?;

    let listing_id = listing.id;

    Ok(Response::default()
        .add_attribute("action", "delist")
        .add_attribute("id", listing_id)
        .add_attribute("seller", info.sender.to_string()))
}
pub fn reserve<TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    until: Expiration,
) -> Result<Response<TCustomResponseMsg>, ContractError> {
    todo!()
}
pub fn buy<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    recipient: Option<String>,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: Cw721CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or(ContractError::ListingNotFound { id: id.clone() })?;

    let price = listing.price.clone();
    let seller = listing.seller.clone();

    let payment = info
        .funds
        .iter()
        .find(|coin| coin.denom == price.denom)
        .ok_or_else(|| ContractError::NoPayment {})?;

    if payment.amount.lt(&price.amount) || payment.denom != price.denom {
        return Err(ContractError::InvalidPayment {
            price: payment.amount.u128(),
            denom: payment.denom.clone(),
        });
    }

    let buyer = match recipient {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => info.sender.clone(),
    };
    let reserved_listing = asset_config
        .reserved_listings
        .may_load(deps.storage, &id)?;
    if let Some(reserved_listing) = reserved_listing {
        if let Some(until) = reserved_listing.reserved_until {
            if !until.is_expired(&env.block) {
                return Err(ContractError::ReservedAsset { id: id.clone() });
            }
            // else the reservation has expired, we can buy
            asset_config.reserved_listings.remove(deps.storage, &id)?;
        }
    }

    let mut nft_info = listing.nft_info.clone();

    nft_info.owner = buyer.clone();
    nft_info.approvals.clear();
    asset_config
        .cw721_config
        .nft_info
        .save(deps.storage, &id, &nft_info)?;

    asset_config.listings.remove(deps.storage, &id)?;
    let mut response = Response::default();

    if !seller.eq(&env.contract.address) {
        response = response.add_message(BankMsg::Send {
            to_address: seller.to_string(),
            amount: vec![payment.clone()], // we send the entire payment to the seller
        });
    }
    Ok(response
        .add_attribute("action", "buy")
        .add_attribute("id", id)
        .add_attribute("price", price.amount.to_string())
        .add_attribute("denom", price.denom)
        .add_attribute("seller", seller.to_string())
        .add_attribute("buyer", buyer.to_string()))
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
        assert!(stored.reserved_until.is_none());
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
        assert!(stored.reserved_until.is_none());
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
        assert!(stored.reserved_until.is_none());
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

#[test]
fn test_buy() {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{Empty, coin, coins};

    // successful buy transfers ownership, pays seller, removes listing, and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let buyer_addr = deps.api.addr_make("buyer");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-1", &nft_info)
            .unwrap();
        let price = coin(100 as u128, "uxion");
        AssetConfig::<Empty>::default()
            .listings
            .save(
                deps.as_mut().storage,
                "token-1",
                &ListingInfo {
                    id: "token-1".to_string(),
                    seller: seller_addr.clone(),
                    price: price.clone(),
                    reserved_until: None,
                    nft_info: nft_info.clone(),
                },
            )
            .unwrap();

        let response = buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[price.clone()]),
            "token-1".to_string(),
            None,
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
                ("action".to_string(), "buy".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), price.denom.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
                ("buyer".to_string(), buyer_addr.to_string()),
            ],
        );
        let msgs: Vec<CosmosMsg<Empty>> = response
            .messages
            .iter()
            .map(|msg| msg.msg.clone())
            .collect();
        assert_eq!(
            msgs,
            vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: seller_addr.to_string(),
                amount: coins(100 as u128, "uxion"),
            })],
        );

        let stored_nft_info = AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .load(deps.as_ref().storage, "token-1")
            .unwrap();
        assert_eq!(stored_nft_info.owner, buyer_addr);
        assert!(stored_nft_info.approvals.is_empty());
        assert!(
            AssetConfig::<Empty>::default()
                .listings
                .may_load(deps.as_ref().storage, "token-1")
                .unwrap()
                .is_none()
        );
    }
    // insufficient payment is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let buyer_addr = deps.api.addr_make("buyer");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-2", &nft_info)
            .unwrap();
        let price = coin(100 as u128, "uxion");
        AssetConfig::<Empty>::default()
            .listings
            .save(
                deps.as_mut().storage,
                "token-2",
                &ListingInfo {
                    id: "token-2".to_string(),
                    seller: seller_addr.clone(),
                    price: price.clone(),
                    reserved_until: None,
                    nft_info: nft_info.clone(),
                },
            )
            .unwrap();

        let err = buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[coin(50 as u128, "uxion")]),
            "token-2".to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidPayment {
                price: 50,
                denom: "uxion".to_string()
            }
        );
    }
    // non-existent listing is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let buyer_addr = deps.api.addr_make("buyer");
        let err = buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[coin(100 as u128, "uxion")]),
            "token-3".to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-3".to_string()
            }
        );
    }
    // reserved assets cannot be bought
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let buyer_addr = deps.api.addr_make("buyer");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-4", &nft_info)
            .unwrap();
        let price = coin(100 as u128, "uxion");
        AssetConfig::<Empty>::default()
            .listings
            .save(
                deps.as_mut().storage,
                "token-4",
                &ListingInfo {
                    id: "token-4".to_string(),
                    seller: seller_addr.clone(),
                    price: price.clone(),
                    reserved_until: Some(Expiration::AtHeight(env.block.height + 100)),
                    nft_info: nft_info.clone(),
                },
            )
            .unwrap();

        let err = buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[price.clone()]),
            "token-4".to_string(),
            None,
        )
        .unwrap_err();
        assert_eq!(err, ContractError::ReservedAsset { id: "token-4".to_string() });
    }
}

#[test]
fn test_delist() {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{Coin, Empty};

    // successful delist removes listing and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
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
        AssetConfig::<Empty>::default()
            .listings
            .save(
                deps.as_mut().storage,
                "token-1",
                &ListingInfo {
                    id: "token-1".to_string(),
                    seller: seller_addr.clone(),
                    price: price.clone(),
                    reserved_until: None,
                    nft_info: nft_info.clone(),
                },
            )
            .unwrap();

        let response = delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-1".to_string(),
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
                ("action".to_string(), "delist".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("seller".to_string(), seller_addr.to_string()),
            ],
        );

        assert!(
            AssetConfig::<Empty>::default()
                .listings
                .may_load(deps.as_ref().storage, "token-1")
                .unwrap()
                .is_none()
        );
    }

    // non-seller cannot delist
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let intruder_addr = deps.api.addr_make("intruder");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-2", &nft_info)
            .unwrap();
        let price = Coin::new(100 as u128, "uxion");
        AssetConfig::<Empty>::default()
            .listings
            .save(
                deps.as_mut().storage,
                "token-2",
                &ListingInfo {
                    id: "token-2".to_string(),
                    seller: seller_addr.clone(),
                    price: price.clone(),
                    reserved_until: None,
                    nft_info: nft_info.clone(),
                },
            )
            .unwrap();
        let err = delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-2".to_string(),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }
    // non-existent listing is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let err = delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-3".to_string(),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-3".to_string()
            }
        );
    }
    // reserved assets cannot be delisted
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        AssetConfig::<Empty>::default()
            .cw721_config
            .nft_info
            .save(deps.as_mut().storage, "token-4", &nft_info)
            .unwrap();
        let price = Coin::new(100 as u128, "uxion");
        AssetConfig::<Empty>::default()
            .listings
            .save(
                deps.as_mut().storage,
                "token-4",
                &ListingInfo {
                    id: "token-4".to_string(),
                    seller: seller_addr.clone(),
                    price: price.clone(),
                    reserved_until: Some(Expiration::AtHeight(env.block.height + 100)),
                    nft_info: nft_info.clone(),
                },
            )
            .unwrap();

        let err = delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-4".to_string(),
        )
        .unwrap_err();
        assert_eq!(err, ContractError::ReservedAsset { id: "token-4".to_string() });
    }
}

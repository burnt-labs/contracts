use cosmwasm_std::{BankMsg, Coin, CustomMsg, Deps, DepsMut, Env, MessageInfo, Response};
use cw721::Expiration;
use cw721::{state::NftInfo, traits::Cw721State};

use crate::{
    error::ContractError,
    state::{AssetConfig, ListingInfo, Reserve},
};

pub fn list<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    price: Coin,
    reservation: Option<Reserve>,
    marketplace_fee_bps: Option<u16>,
    marketplace_fee_recipient: Option<String>,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();
    // make sure the caller is the owner of the token
    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    // check if we can list the asset
    check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;
    // make sure the price is greater than zero
    if price.amount.is_zero() {
        return Err(ContractError::InvalidListingPrice {
            price: price.amount.u128(),
        });
    }

    let (validated_marketplace_fee_bps, validated_marketplace_fee_recipient) =
        match (marketplace_fee_bps, marketplace_fee_recipient) {
            (Some(bps), Some(recipient)) => {
                if bps > 10_000 {
                    return Err(ContractError::InvalidMarketplaceFee { bps, recipient });
                }
                let recipient_addr = deps.api.addr_validate(&recipient)?;
                (Some(bps), Some(recipient_addr))
            }
            (Some(bps), None) => {
                return Err(ContractError::InvalidMarketplaceFee {
                    bps,
                    recipient: "".to_string(),
                });
            }
            (None, Some(recipient)) => {
                return Err(ContractError::InvalidMarketplaceFee { bps: 0, recipient });
            }
            (None, None) => (None, None),
        };
    // Ensure the listing does not already exist
    let old_listing = asset_config.listings.may_load(deps.storage, &id)?;
    if old_listing.is_some() {
        return Err(ContractError::ListingAlreadyExists { id });
    }
    // Save the listing
    let listing = ListingInfo {
        id: id.clone(),
        seller: nft_info.owner.clone(),
        price: price.clone(),
        reserved: reservation.clone(),
        marketplace_fee_bps: validated_marketplace_fee_bps,
        marketplace_fee_recipient: validated_marketplace_fee_recipient,
    };
    asset_config.listings.save(deps.storage, &id, &listing)?;
    Ok(Response::default()
        .add_attribute("action", "list")
        .add_attribute("id", id)
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("price", price.amount.to_string())
        .add_attribute("denom", price.denom.to_string())
        .add_attribute("seller", nft_info.owner.clone().to_string())
        .add_attribute(
            "reserved_until",
            reservation.map_or("none".to_string(), |r| r.reserved_until.to_string()),
        ))
}
pub fn delist<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or_else(|| ContractError::ListingNotFound { id: id.clone() })?;

    // only the ones who can list can delist
    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;

    asset_config.listings.remove(deps.storage, &id)?;

    Ok(Response::default()
        .add_attribute("action", "delist")
        .add_attribute("id", listing.id)
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("seller", listing.seller.to_string()))
}
pub fn reserve<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    reservation: Reserve,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let mut listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or_else(|| ContractError::ListingNotFound { id: id.clone() })?;

    // only the ones who can list can reserve
    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;
    check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;

    if let Some(reserved) = &listing.reserved {
        if !Expiration::AtTime(reserved.reserved_until).is_expired(&env.block) {
            return Err(ContractError::ReservedAsset { id: id.clone() });
        }
    }

    listing.reserved = Some(Reserve {
        reserver: reservation.reserver.clone(),
        reserved_until: reservation.reserved_until,
    });
    asset_config.listings.save(deps.storage, &id, &listing)?;

    Ok(Response::default()
        .add_attribute("action", "reserve")
        .add_attribute("id", id)
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("reserver", reservation.reserver.to_string())
        .add_attribute("reserved_until", reservation.reserved_until.to_string()))
}
pub fn unreserve<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    id: String,
    delist: bool,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let mut listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or_else(|| ContractError::ListingNotFound { id: id.clone() })?;

    let reserved = listing
        .reserved
        .as_ref()
        .ok_or_else(|| ContractError::ReservationNotFound { id: id.clone() })?;

    let nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;

    if reserved.reserver != info.sender {
        check_can_list(deps.as_ref(), env, info.sender.as_ref(), &nft_info)?;
    }

    let response = Response::<TCustomResponseMsg>::default()
        .add_attribute("action", "unreserve")
        .add_attribute("id", id.clone())
        .add_attribute("collection", env.contract.address.clone())
        .add_attribute("reserver", info.sender.to_string());

    if delist {
        asset_config.listings.remove(deps.storage, &id)?;
        return Ok(response.add_attribute("delisted", "true"));
    }

    listing.reserved = None;
    asset_config.listings.save(deps.storage, &id, &listing)?;

    Ok(response.add_attribute("delisted", "false"))
}
pub fn buy<TNftExtension, TCustomResponseMsg>(
    deps: DepsMut,
    _env: &Env,
    info: &MessageInfo,
    id: String,
    recipient: Option<String>,
    deductions: Vec<(String, Coin, String)>,
) -> Result<Response<TCustomResponseMsg>, ContractError>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    let asset_config = AssetConfig::<TNftExtension>::default();

    let listing = asset_config
        .listings
        .may_load(deps.storage, &id)?
        .ok_or(ContractError::ListingNotFound { id: id.clone() })?;

    let price = listing.price.clone();
    let seller = listing.seller.clone();

    let mut payment = info
        .funds
        .iter()
        .find(|coin| coin.denom == price.denom)
        .ok_or_else(|| ContractError::NoPayment {})?
        .clone();

    if payment.amount.lt(&price.amount) || payment.denom != price.denom {
        return Err(ContractError::InvalidPayment {
            price: payment.amount.u128(),
            denom: payment.denom.clone(),
        });
    }

    let mut response = Response::<TCustomResponseMsg>::default();

    if let Some(market_fee) = listing.marketplace_fee_bps {
        let fee_amount = payment
            .amount
            .checked_multiply_ratio(market_fee, 10_000_u128)
            .map_err(|_| ContractError::InsufficientFunds {})?;
        payment.amount = payment
            .amount
            .checked_sub(fee_amount)
            .map_err(|_| ContractError::InsufficientFunds {})?;
        if let Some(recipient) = listing.marketplace_fee_recipient {
            if !fee_amount.is_zero() {
                response = response.add_attribute("marketplace_fee", fee_amount.to_string());
                response = response.add_message(BankMsg::Send {
                    to_address: recipient.to_string(),
                    amount: vec![Coin {
                        denom: payment.denom.clone(),
                        amount: fee_amount,
                    }],
                });
            }
        }
    }

    // remove all other deductions e.g. royalties from payment
    for (_, amount, _) in deductions {
        payment.amount = payment
            .amount
            .checked_sub(amount.amount)
            .map_err(|_| ContractError::InsufficientFunds {})?;
    }

    let buyer = match recipient {
        Some(addr) => deps.api.addr_validate(&addr)?,
        None => info.sender.clone(),
    };

    if let Some(reserved) = listing.reserved {
        if reserved.reserver != info.sender && reserved.reserver != buyer {
            return Err(ContractError::Unauthorized {});
        }
    }

    let mut nft_info = asset_config.cw721_config.nft_info.load(deps.storage, &id)?;

    nft_info.owner = buyer.clone();
    nft_info.approvals.clear();
    asset_config
        .cw721_config
        .nft_info
        .save(deps.storage, &id, &nft_info)?;

    asset_config.listings.remove(deps.storage, &id)?;

    response = response
        .add_message(BankMsg::Send {
            to_address: seller.to_string(),
            amount: vec![payment.clone()], // we send the remaining payment after deductions to the seller
        })
        .add_attribute("action", "buy")
        .add_attribute("id", id)
        .add_attribute("price", price.amount.to_string())
        .add_attribute("denom", price.denom)
        .add_attribute("seller", seller.to_string())
        .add_attribute("buyer", buyer.to_string());
    Ok(response)
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

#[cfg(test)]
fn expect_ok<T, E: core::fmt::Debug>(res: Result<T, E>) -> T {
    match res {
        Ok(value) => value,
        Err(err) => panic!("expected Ok(..) but got Err({:?})", err),
    }
}

#[cfg(test)]
fn expect_err<T, E: core::fmt::Debug>(res: Result<T, E>) -> E {
    match res {
        Ok(_) => panic!("expected Err(..) but got Ok(..)"),
        Err(err) => err,
    }
}

#[cfg(test)]
fn expect_some<T>(opt: Option<T>) -> T {
    match opt {
        Some(value) => value,
        None => panic!("expected Some(..) but got None"),
    }
}

#[test]
fn test_list() {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{Coin, Empty, StdError};

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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &nft_info,
        ));

        let price = Coin::new(100_u128, "uxion");
        let response = expect_ok(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            price.clone(),
            None,
            None,
            None,
        ));

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
                ("collection".to_string(), env.contract.address.to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), owner_addr.to_string()),
                ("reserved_until".to_string(), "none".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-1"),
        );
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_none());

        let duplicate_err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            Coin::new(200_u128, "uxion"),
            None,
            None,
            None,
        ));
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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &nft_info,
        ));

        let intruder_addr = deps.api.addr_make("intruder");
        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-2".to_string(),
            Coin::new(100_u128, "uxion"),
            None,
            None,
            None,
        ));
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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-3",
            &nft_info,
        ));

        let price = Coin::new(100_u128, "uxion");
        let response = expect_ok(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&approver_addr, &[]),
            "token-3".to_string(),
            price.clone(),
            None,
            None,
            None,
        ));

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
                ("collection".to_string(), env.contract.address.to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), owner_addr.to_string()),
                ("reserved_until".to_string(), "none".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-3"),
        );
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_none());
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
        expect_ok(asset_config.cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &nft_info,
        ));
        expect_ok(asset_config.cw721_config.operators.save(
            deps.as_mut().storage,
            (&owner_addr, &operator_addr),
            &cw721::Expiration::AtHeight(env.block.height + 100),
        ));

        let price = Coin::new(100_u128, "uxion");
        let response = expect_ok(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&operator_addr, &[]),
            "token-4".to_string(),
            price.clone(),
            None,
            None,
            None,
        ));

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
                ("collection".to_string(), env.contract.address.to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), "uxion".to_string()),
                ("seller".to_string(), owner_addr.to_string()),
                ("reserved_until".to_string(), "none".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-4"),
        );
        assert_eq!(stored.price, price);
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_none());
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
        expect_ok(asset_config.cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-5",
            &nft_info,
        ));
        expect_ok(asset_config.cw721_config.operators.save(
            deps.as_mut().storage,
            (&owner_addr, &exp_approval_addr),
            &cw721::Expiration::AtHeight(env.block.height - 1),
        ));

        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&exp_approval_addr, &[]),
            "token-5".to_string(),
            Coin::new(100_u128, "uxion"),
            None,
            None,
            None,
        ));
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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-3",
            &nft_info,
        ));

        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-3".to_string(),
            Coin::new(0_u128, "uxion"),
            None,
            None,
            None,
        ));
        assert_eq!(err, ContractError::InvalidListingPrice { price: 0 });
    }
    // non-existent item cannot be listed
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");

        let err = expect_err(list::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-999".to_string(),
            Coin::new(100_u128, "uxion"),
            None,
            None,
            None,
        ));
        match err {
            ContractError::Std(StdError::NotFound { .. }) => {}
            _ => panic!("expected NotFound error"),
        }
    }
}

#[test]
fn test_buy() {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{CosmosMsg, Empty, coin, coins};

    // successful buy transfers ownership, pays seller, pays royalties, removes listing, and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let buyer_addr = deps.api.addr_make("buyer");
        let owner_addr = deps.api.addr_make("owner");
        let nft_info = NftInfo {
            owner: seller_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &nft_info,
        ));
        let price = coin(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: None,
                marketplace_fee_bps: Some(100),
                marketplace_fee_recipient: Some(seller_addr.clone()),
            },
        ));

        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[price.clone()]),
            "token-1".to_string(),
            None,
            vec![(
                owner_addr.to_string(),
                coin(10_u128, "uxion"),
                "royalties".to_string(),
            )],
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("marketplace_fee".to_string(), "1".to_string()),
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
            vec![
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: seller_addr.to_string(),
                    amount: coins(1_u128, "uxion"),
                }),
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: seller_addr.to_string(),
                    amount: coins(89_u128, "uxion"),
                })
            ],
        );

        let stored_nft_info = expect_ok(
            AssetConfig::<Empty>::default()
                .cw721_config
                .nft_info
                .load(deps.as_ref().storage, "token-1"),
        );
        assert_eq!(stored_nft_info.owner, buyer_addr);
        assert!(stored_nft_info.approvals.is_empty());
        assert!(
            expect_ok(
                AssetConfig::<Empty>::default()
                    .listings
                    .may_load(deps.as_ref().storage, "token-1")
            )
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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &nft_info,
        ));
        let price = coin(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-2",
            &ListingInfo {
                id: "token-2".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: None,
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[coin(50_u128, "uxion")]),
            "token-2".to_string(),
            None,
            vec![],
        ));
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
        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[coin(100_u128, "uxion")]),
            "token-3".to_string(),
            None,
            vec![],
        ));
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-3".to_string()
            }
        );
    }
    // reserved assets can only be bought by reserver
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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &nft_info,
        ));
        let price = coin(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-4",
            &ListingInfo {
                id: "token-4".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: Some(Reserve {
                    reserver: buyer_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let err = expect_err(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[price.clone()]),
            "token-4".to_string(),
            None,
            vec![],
        ));
        assert_eq!(err, ContractError::Unauthorized {});

        // successful buy by reserver
        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&buyer_addr, &[price.clone()]),
            "token-4".to_string(),
            None,
            vec![],
        ));
        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "buy".to_string()),
                ("id".to_string(), "token-4".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), price.denom.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
                ("buyer".to_string(), buyer_addr.to_string()),
            ],
        );
        // successful buy on behalf of reserver
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-4",
            &ListingInfo {
                id: "token-4".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: Some(Reserve {
                    reserver: buyer_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));
        let mut deps = deps;
        let response = expect_ok(buy::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[price.clone()]),
            "token-4".to_string(),
            Some(buyer_addr.to_string()), // buyer is the reserver
            vec![],
        ));
        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "buy".to_string()),
                ("id".to_string(), "token-4".to_string()),
                ("price".to_string(), price.amount.to_string()),
                ("denom".to_string(), price.denom.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
                ("buyer".to_string(), buyer_addr.to_string()),
            ],
        );
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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &nft_info,
        ));
        let price = Coin::new(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: None,
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let response = expect_ok(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-1".to_string(),
        ));

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
                ("collection".to_string(), env.contract.address.to_string()),
                ("seller".to_string(), seller_addr.to_string()),
            ],
        );

        assert!(
            expect_ok(
                AssetConfig::<Empty>::default()
                    .listings
                    .may_load(deps.as_ref().storage, "token-1")
            )
            .is_none()
        );
    }

    // non-admin cannot delist
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
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &nft_info,
        ));
        let price = Coin::new(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-2",
            &ListingInfo {
                id: "token-2".to_string(),
                seller: seller_addr.clone(),
                price: price.clone(),
                reserved: None,
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));
        let err = expect_err(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-2".to_string(),
        ));
        assert_eq!(err, ContractError::Unauthorized {});
    }
    // non-existent listing is rejected
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let seller_addr = deps.api.addr_make("seller");
        let err = expect_err(delist::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&seller_addr, &[]),
            "token-3".to_string(),
        ));
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-3".to_string()
            }
        );
    }
}

#[test]
fn test_reserve() {
    use cosmwasm_std::Empty;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};

    // successful reserve stores state and emits attributes
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let buyer_addr = deps.api.addr_make("buyer");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &nft_info,
        ));

        // cannot reserve unlisted item
        let reservation = Reserve {
            reserver: buyer_addr.clone(),
            reserved_until: env.block.time.plus_seconds(600),
        };
        let err = expect_err(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            reservation.clone(),
        ));
        assert_eq!(
            err,
            ContractError::ListingNotFound {
                id: "token-1".to_string()
            }
        );
        // list item first
        let price = cosmwasm_std::Coin::new(100_u128, "uxion");
        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: owner_addr.clone(),
                price: price.clone(),
                reserved: None,
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));
        let response = expect_ok(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            reservation.clone(),
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "reserve".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("reserver".to_string(), buyer_addr.to_string()),
                (
                    "reserved_until".to_string(),
                    reservation.reserved_until.to_string()
                ),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-1"),
        );
        assert_eq!(stored.seller, owner_addr);
        assert!(stored.reserved.is_some());
        let reserved = expect_some(stored.reserved);
        assert_eq!(reserved.reserver, buyer_addr);
        assert_eq!(reserved.reserved_until, reservation.reserved_until);

        // cannot reserve already reserved item
        let err = expect_err(reserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&owner_addr, &[]),
            "token-1".to_string(),
            reservation.clone(),
        ));
        assert_eq!(
            err,
            ContractError::ReservedAsset {
                id: "token-1".to_string()
            }
        );
    }
}

#[test]
fn test_unreserve() {
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{Coin, Empty};

    // reserver can remove reservation while keeping listing active
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-1",
            &nft_info,
        ));

        let reservation = Reserve {
            reserver: reserver_addr.clone(),
            reserved_until: env.block.time.plus_seconds(600),
        };

        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-1",
            &ListingInfo {
                id: "token-1".to_string(),
                seller: owner_addr.clone(),
                price: Coin::new(100_u128, "uxion"),
                reserved: Some(reservation.clone()),
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let response = expect_ok(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-1".to_string(),
            false,
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "unreserve".to_string()),
                ("id".to_string(), "token-1".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("reserver".to_string(), reserver_addr.to_string()),
                ("delisted".to_string(), "false".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .load(deps.as_ref().storage, "token-1"),
        );
        assert!(stored.reserved.is_none());
    }

    // reserver can delist when requested
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-2",
            &nft_info,
        ));

        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-2",
            &ListingInfo {
                id: "token-2".to_string(),
                seller: owner_addr.clone(),
                price: Coin::new(150_u128, "uxion"),
                reserved: Some(Reserve {
                    reserver: reserver_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let response = expect_ok(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-2".to_string(),
            true,
        ));

        let attrs: Vec<(String, String)> = response
            .attributes
            .iter()
            .map(|attr| (attr.key.clone(), attr.value.clone()))
            .collect();
        assert_eq!(
            attrs,
            vec![
                ("action".to_string(), "unreserve".to_string()),
                ("id".to_string(), "token-2".to_string()),
                ("collection".to_string(), env.contract.address.to_string()),
                ("reserver".to_string(), reserver_addr.to_string()),
                ("delisted".to_string(), "true".to_string()),
            ],
        );

        let stored = expect_ok(
            AssetConfig::<Empty>::default()
                .listings
                .may_load(deps.as_ref().storage, "token-2"),
        );
        assert!(stored.is_none());
    }

    // non-reserver cannot unreserve
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let intruder_addr = deps.api.addr_make("intruder");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-3",
            &nft_info,
        ));

        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-3",
            &ListingInfo {
                id: "token-3".to_string(),
                seller: owner_addr.clone(),
                price: Coin::new(200_u128, "uxion"),
                reserved: Some(Reserve {
                    reserver: reserver_addr.clone(),
                    reserved_until: env.block.time.plus_seconds(600),
                }),
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let err = expect_err(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&intruder_addr, &[]),
            "token-3".to_string(),
            false,
        ));
        assert_eq!(err, ContractError::Unauthorized {});
    }

    // cannot unreserve when listing not reserved
    {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let owner_addr = deps.api.addr_make("owner");
        let reserver_addr = deps.api.addr_make("reserver");
        let nft_info = NftInfo {
            owner: owner_addr.clone(),
            approvals: vec![],
            token_uri: None,
            extension: Empty {},
        };
        expect_ok(AssetConfig::<Empty>::default().cw721_config.nft_info.save(
            deps.as_mut().storage,
            "token-4",
            &nft_info,
        ));

        expect_ok(AssetConfig::<Empty>::default().listings.save(
            deps.as_mut().storage,
            "token-4",
            &ListingInfo {
                id: "token-4".to_string(),
                seller: owner_addr.clone(),
                price: Coin::new(250_u128, "uxion"),
                reserved: None,
                marketplace_fee_bps: None,
                marketplace_fee_recipient: None,
            },
        ));

        let err = expect_err(unreserve::<Empty, Empty>(
            deps.as_mut(),
            &env,
            &message_info(&reserver_addr, &[]),
            "token-4".to_string(),
            false,
        ));
        assert_eq!(
            err,
            ContractError::ReservationNotFound {
                id: "token-4".to_string()
            }
        );
    }
}

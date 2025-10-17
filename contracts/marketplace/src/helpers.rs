use crate::error::ContractError;
use crate::state::CONFIG;
use asset::msg::AssetExtensionExecuteMsg as AssetExecuteMsg;
use asset::msg::AssetExtensionQueryMsg;
use asset::msg::QueryMsg as AssetQueryMsg;
use asset::state::ListingInfo;
use blake2::{Blake2s256, Digest};
use cosmwasm_std::{ensure, ensure_eq, Coin};
use cosmwasm_std::{Addr, DepsMut, Empty, MessageInfo, QuerierWrapper};
use cw721::msg::OwnerOfResponse;
use cw721_base::msg::QueryMsg;
use cw_utils::one_coin;

pub fn only_owner(
    querier: &QuerierWrapper,
    info: &MessageInfo,
    collection: &Addr,
    token_id: &str,
) -> Result<(), ContractError> {
    let result = querier.query_wasm_smart::<OwnerOfResponse>(
        collection.clone(),
        &QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
            include_expired: Some(false),
        },
    );
    match result {
        Ok(owner_resp) => {
            if owner_resp.owner != info.sender.to_string() {
                return Err(ContractError::Unauthorized {
                    message: "sender is not owner".to_string(),
                });
            }
            Ok(())
        }
        Err(_) => Err(ContractError::Unauthorized {
            message: "sender is not owner".to_string(),
        }),
    }
}

pub fn only_manager(info: &MessageInfo, deps: &DepsMut) -> Result<(), ContractError> {
    let manager = CONFIG.load(deps.storage)?.manager;
    ensure_eq!(
        info.sender,
        manager,
        ContractError::Unauthorized {
            message: "sender is not manager".to_string()
        }
    );
    Ok(())
}

pub fn query_listing(
    querier: &QuerierWrapper,
    collection: &Addr,
    token_id: &str,
) -> Result<ListingInfo, ContractError> {
    if let Ok(listing) = querier.query_wasm_smart::<ListingInfo>(
        collection.clone(),
        &AssetQueryMsg::<Empty, Empty, AssetExtensionQueryMsg>::Extension {
            msg: AssetExtensionQueryMsg::GetListing {
                token_id: token_id.to_string(),
            },
        },
    ) {
        Ok(listing)
    } else {
        Err(ContractError::NotListed {})
    }
}
pub fn not_listed(
    querier: &QuerierWrapper,
    collection: &Addr,
    token_id: &str,
) -> Result<(), ContractError> {
    let listing_response = query_listing(querier, collection, token_id);
    match listing_response {
        Ok(_) => Err(ContractError::AlreadyListed {}),
        Err(_) => Ok(()),
    }
}

pub fn generate_id(parts: Vec<&[u8]>) -> String {
    let mut hasher = Blake2s256::new();
    for part in parts {
        hasher.update(part);
    }
    format!("{:x}", hasher.finalize())
}

pub fn valid_payment(
    info: &MessageInfo,
    price: Coin,
    valid_denom: String,
) -> Result<(), ContractError> {
    let payment = one_coin(info)?;
    // check if the payment is the valid denom
    ensure_eq!(
        payment.denom,
        valid_denom,
        ContractError::InvalidListingDenom {
            expected: valid_denom,
            actual: payment.denom,
        }
    );
    // check if the payment  and listing have the same denom
    ensure_eq!(
        payment.denom,
        price.denom,
        ContractError::InvalidListingDenom {
            expected: price.denom,
            actual: payment.denom,
        }
    );
    // check if the payment is the same amount as the price
    ensure!(
        payment.amount == price.amount,
        ContractError::InvalidPayment {
            expected: price,
            actual: payment,
        }
    );
    Ok(())
}

pub fn asset_list_msg(
    token_id: String,
    price: Coin,
    marketplace_fee_bps: Option<u16>,
    marketplace_fee_recipient: Option<String>,
) -> asset::msg::ExecuteMsg<
    cw721::DefaultOptionalNftExtensionMsg,
    cw721::DefaultOptionalCollectionExtensionMsg,
    AssetExecuteMsg,
> {
    asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        AssetExecuteMsg,
    >::UpdateExtension {
        msg: AssetExecuteMsg::List {
            token_id: token_id.clone(),
            price: price.clone(),
            reservation: None,
            marketplace_fee_bps,
            marketplace_fee_recipient,
        },
    }
}

pub fn asset_buy_msg(
    recipient: Addr,
    token_id: String,
) -> asset::msg::ExecuteMsg<
    cw721::DefaultOptionalNftExtensionMsg,
    cw721::DefaultOptionalCollectionExtensionMsg,
    AssetExecuteMsg,
> {
    asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        AssetExecuteMsg,
    >::UpdateExtension {
        msg: AssetExecuteMsg::Buy {
            token_id: token_id.clone(),
            recipient: Some(recipient.to_string()),
        },
    }
}

pub fn asset_reserve_msg(
    token_id: String,
    reserver: Addr,
    reserved_until: cw721::Expiration,
) -> asset::msg::ExecuteMsg<
    cw721::DefaultOptionalNftExtensionMsg,
    cw721::DefaultOptionalCollectionExtensionMsg,
    AssetExecuteMsg,
> {
    asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        AssetExecuteMsg,
    >::UpdateExtension {
        msg: AssetExecuteMsg::Reserve {
            token_id,
            reservation: asset::state::Reserve {
                reserver,
                reserved_until,
            },
        },
    }
}

pub fn asset_delist_msg(
    token_id: String,
) -> asset::msg::ExecuteMsg<
    cw721::DefaultOptionalNftExtensionMsg,
    cw721::DefaultOptionalCollectionExtensionMsg,
    AssetExecuteMsg,
> {
    asset::msg::ExecuteMsg::<
        cw721::DefaultOptionalNftExtensionMsg,
        cw721::DefaultOptionalCollectionExtensionMsg,
        AssetExecuteMsg,
    >::UpdateExtension {
        msg: AssetExecuteMsg::Delist { token_id },
    }
}

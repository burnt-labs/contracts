use asset::msg::AssetExtensionQueryMsg;
use asset::msg::QueryMsg as AssetQueryMsg;
use asset::state::ListingInfo;
use blake2::{Blake2s256, Digest};
use cosmwasm_std::{Addr, Empty, MessageInfo, QuerierWrapper};
use cw721::msg::OwnerOfResponse;
use cw721_base::msg::QueryMsg;

use crate::error::ContractError;

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
        Err(_) => {
            return Err(ContractError::Unauthorized {
                message: "sender is not owner".to_string(),
            })
        }
    }
}
pub fn query_listing(
    querier: &QuerierWrapper,
    collection: &Addr,
    token_id: &str,
) -> Result<ListingInfo<cw721::DefaultOptionalNftExtension>, ContractError> {
    if let Ok(listing) = querier
        .query_wasm_smart::<ListingInfo<cw721::DefaultOptionalNftExtension>>(
            collection.clone(),
            &AssetQueryMsg::<Empty, Empty, AssetExtensionQueryMsg>::Extension {
                msg: AssetExtensionQueryMsg::GetListing {
                    token_id: token_id.to_string(),
                },
            },
        )
    {
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

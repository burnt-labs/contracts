use cosmwasm_std::{CustomMsg, Deps, DepsMut, Env, MessageInfo, Response};
use cw721::{state::NftInfo, traits::Cw721State};

use crate::{
    error::ContractError,
    plugin::{Plugin, PluginCtx},
    state::{AssetConfig, ListingInfo},
};

/// Default implementation of asset contract execute msgs
/// The response from these methods can include custom messages to be executed after the main action
/// is performed.
pub trait XionAssetExecuteExtension<TCustomResponseMsg, TNftExtension, TPluginContext>
where
    TNftExtension: Cw721State,
    TCustomResponseMsg: CustomMsg,
{
    fn list(
        &self,
        deps: DepsMut,
        _env: &Env,
        info: &MessageInfo,
        id: String,
        price: u128,
        nft_info: NftInfo<TNftExtension>,
        ctx: &mut PluginCtx<TPluginContext, TCustomResponseMsg>,
    ) -> Result<Response<TCustomResponseMsg>, ContractError>
    where
        NftInfo<TNftExtension>: Plugin<TPluginContext, TCustomResponseMsg>,
    {
        // make sure the caller is the owner of the token
        if info.sender != nft_info.owner {
            return Err(ContractError::Unauthorized {});
        }
        // make sure the price is greater than zero
        if price == 0 {
            return Err(ContractError::InvalidListingPrice { price });
        }
        let asset_config = AssetConfig::<TNftExtension>::default();
        // Ensure the listing does not already exist
        let old_listing = asset_config.listings.may_load(deps.storage, &id)?;
        if old_listing.is_some() {
            return Err(ContractError::ListingAlreadyExists { id });
        }
        // check if we can list the asset
        Self::check_can_list(
            ctx.deps.as_ref(),
            &ctx.env,
            ctx.info.sender.as_ref(),
            &nft_info,
        )?;
        // run the plugins
        let response = nft_info.run_plugin(ctx)?;
        // Save the listing
        let listing = ListingInfo {
            id: id.clone(),
            seller: info.sender.clone(),
            price,
            is_frozen: false,
            nft_info,
        };
        asset_config.listings.save(deps.storage, &id, &listing)?;
        Ok(response
            .add_attribute("action", "list")
            .add_attribute("id", id)
            .add_attribute("price", price.to_string())
            .add_attribute("seller", info.sender.clone().to_string()))
    }
    fn delist(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        id: String,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        todo!()
    }
    fn freeze_listing(
        &self,
        deps: DepsMut,
        env: &Env,
        info: &MessageInfo,
        id: String,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        todo!()
    }
    fn buy(
        &self,
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
    fn check_can_list(
        deps: Deps,
        env: &Env,
        sender: &str,
        token: &NftInfo<TNftExtension>,
        collection_operators: 
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
        let config = AssetConfig::<TNftExtension>::default();
        let op = config
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
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        attr,
        testing::{message_info, mock_dependencies, mock_env, mock_info},
    };
    use cw721::{
        NftExtension,
        msg::NftExtensionMsg,
        state::{NftInfo, Trait},
        traits::Cw721Execute,
    };

    use crate::{
        CONTRACT_NAME, CONTRACT_VERSION,
        contract::AssetContract,
        msg::{ExecuteMsg, InstantiateMsg, XionAssetCollectionMetadataMsg},
        state::XionAssetCollectionMetadata,
    };

    const CREATOR: &str = "creator";

    #[test]
    fn test_listing() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make(CREATOR);
        let info = message_info(&sender, &[]);
        let contract = AssetContract::default();
        let instantiate_msg = InstantiateMsg::<XionAssetCollectionMetadataMsg> {
            name: "xion_asset".to_string(),
            symbol: "XAT".to_string(),
            collection_info_extension: XionAssetCollectionMetadataMsg {
                royalty_bps: Some(500),
                royalty_recipient: Some(deps.api.addr_make("royalty_recipient")),
                royalty_on_primary: Some(true),
                min_list_price: Some(100),
                not_before: None,
                not_after: None,
                plugins: vec![],
            },
            withdraw_address: None,
            minter: None,
            creator: None,
        };
        contract
            .instantiate_with_version(
                deps.as_mut(),
                &env,
                &info,
                instantiate_msg,
                CONTRACT_NAME,
                CONTRACT_VERSION,
            )
            .unwrap();
        let token_uri = Some("https://starships.example.com/Starship/Enterprise.json".into());
        let extension = Some(NftExtensionMsg {
            description: Some("description1".into()),
            name: Some("name1".to_string()),
            attributes: Some(vec![Trait {
                display_type: None,
                trait_type: "type1".to_string(),
                value: "value1".to_string(),
            }]),
            ..NftExtensionMsg::default()
        });
        let owner = deps.api.addr_make("owner");

        for i in 1..=3 {
            let exec_msg = ExecuteMsg::Mint {
                token_id: format!("token{}", i),
                owner: owner.to_string(),
                token_uri: token_uri.clone(),
                extension: extension.clone(),
            };
            contract
                .execute(deps.as_mut(), &mock_env(), &info, exec_msg)
                .unwrap();
        }

        let list_info = message_info(&sender, &[]);
        let list_msg = ExecuteMsg::UpdateExtension {
            msg: crate::msg::XionAssetExtensionExecuteMsg::List {
                id: "token1".to_string(),
                price: 100,
            },
        };
        let res = contract
            .execute(deps.as_mut(), &env, &list_info, list_msg)
            .unwrap();
        assert_eq!(res.attributes[0], attr("action", "list"));
        assert_eq!(res.attributes[1], attr("id", "token1"));
        assert_eq!(res.attributes[2], attr("price", "100"));
        assert_eq!(res.attributes[3], attr("seller", sender.to_string()));

        let listings_by_seller = LISTINGS_TOKEN_INFO
            .idx
            .seller
            .prefix(sender.clone())
            .range(
                deps.as_ref().storage,
                None,
                None,
                cosmwasm_std::Order::Ascending,
            )
            .map(|item| item.unwrap().1)
            .collect::<Vec<_>>();
        assert_eq!(listings_by_seller[0].owner, sender);
    }
}

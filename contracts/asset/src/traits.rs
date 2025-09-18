use cosmwasm_std::{CustomMsg, DepsMut, Env, MessageInfo, Response, to_binary, to_json_binary};
use cw721::{state::NftInfo, NftExtension};

use crate::{
    error::ContractError,
    state::{LISTINGS_TOKEN_INFO, ListingInfo},
};

/// Default implementation of asset contract execute msgs
/// The response from these methods can include custom messages to be executed after the main action
/// is performed.
pub trait XionAssetExecuteExtension<TCustomResponseMsg>
where
    TCustomResponseMsg: CustomMsg,
{
    fn list(
        &self,
        deps: DepsMut,
        _env: &Env,
        info: &MessageInfo,
        id: String,
        price: u128,
    ) -> Result<Response<TCustomResponseMsg>, ContractError> {
        // let listing = ListingInfo {
        //     id: id.clone(),
        //     seller: info.sender.clone(),
        //     price,
        //     is_frozen: false,
        // };
        let listing2 = NftInfo {
            approvals: vec![],
            owner: info.sender.clone(),
            token_uri: None,
            extension: NftExtension {
                description: None,
                name: None,
                attributes: None,
                image: None,
                image_data: None,
                external_url: None,
                background_color: None,
                animation_url: None,
                youtube_url: None,
            },
        };
        LISTINGS_TOKEN_INFO.save(deps.storage, &id, &listing2)?;
        Ok(Response::new()
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
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{attr, testing::{message_info, mock_dependencies, mock_env, mock_info}};
    use cw721::{msg::NftExtensionMsg, state::{NftInfo, Trait}, traits::Cw721Execute, NftExtension};

    use crate::{
        contract::AssetContract, msg::{ExecuteMsg, InstantiateMsg, XionAssetCollectionMetadataMsg}, state::{XionAssetCollectionMetadata, LISTINGS_TOKEN_INFO}, CONTRACT_NAME, CONTRACT_VERSION
    };

    // write a test to instantiate an asset contract with the default implementation and mint 3 items
    // then list 2 of them and get the listings by the seller and confirm the returned listings
    // are nftinfos with the correct ids
    #[test]
    fn test_listings_indexer_uses_nft_pk_namespace() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let sender = deps.api.addr_make("sender");
        let info = message_info(&sender, &[]);
        let contract = AssetContract::default();
        let instantiate_msg = InstantiateMsg::<XionAssetCollectionMetadataMsg> {
            name: "Test".to_string(),
            symbol: "TST".to_string(),
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
            .range(deps.as_ref().storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|item| item.unwrap().1).collect::<Vec<_>>();
        assert_eq!(listings_by_seller[0].owner, sender);
    }
}

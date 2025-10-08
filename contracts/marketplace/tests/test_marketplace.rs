use cosmwasm_std::{coin, Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper};
use serde_json::json;
use xion_nft_marketplace::helpers::generate_id;
use xion_nft_marketplace::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use xion_nft_marketplace::state::{Listing, ListingStatus};

fn asset_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        asset::contracts::asset_base::execute,
        asset::contracts::asset_base::instantiate,
        asset::contracts::asset_base::query,
    ))
}

fn marketplace_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        xion_nft_marketplace::contract::execute,
        xion_nft_marketplace::contract::instantiate,
        xion_nft_marketplace::query::query,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset::msg::InstantiateMsg as AssetInstantiateMsg;
    use cw721::DefaultOptionalCollectionExtensionMsg;
    use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
    use cw_multi_test::{App, Executor};
    use xion_nft_marketplace::helpers::query_listing;

    fn setup_app() -> App {
        App::default()
    }

    fn setup_asset_contract(app: &mut App, minter: &Addr) -> Addr {
        let asset_code_id = app.store_code(asset_contract());

        let instantiate_msg = AssetInstantiateMsg {
            name: "Test Asset".to_string(),
            symbol: "TEST".to_string(),
            minter: Some(minter.to_string()),
            collection_info_extension: DefaultOptionalCollectionExtensionMsg::default(),
            creator: Some(minter.to_string()),
            withdraw_address: None,
        };

        app.instantiate_contract(
            asset_code_id,
            minter.clone(),
            &instantiate_msg,
            &[],
            "test-asset",
            None,
        )
        .unwrap()
    }

    fn setup_marketplace_contract(app: &mut App, manager: &Addr) -> Addr {
        let marketplace_code_id = app.store_code(marketplace_contract());

        let config_json = json!({
            "manager": manager.to_string(),
            "sale_approvals": false,
            "fee_bps": 250,
            "listing_denom": "uxion"
        });

        let instantiate_msg = InstantiateMsg {
            config: serde_json::from_value(config_json).unwrap(),
        };

        app.instantiate_contract(
            marketplace_code_id,
            manager.clone(),
            &instantiate_msg,
            &[],
            "test-marketplace",
            None,
        )
        .unwrap()
    }

    fn mint_nft(app: &mut App, asset_contract: &Addr, minter: &Addr, owner: &Addr, token_id: &str) {
        let mint_msg = asset::msg::ExecuteMsg::<
            cw721::DefaultOptionalNftExtensionMsg,
            cw721::DefaultOptionalCollectionExtensionMsg,
            asset::msg::AssetExtensionExecuteMsg,
        >::Mint {
            token_id: token_id.to_string(),
            owner: owner.to_string(),
            token_uri: Some("https://example.com/metadata.json".to_string()),
            extension: cw721::DefaultOptionalNftExtensionMsg::default(),
        };

        app.execute_contract(minter.clone(), asset_contract.clone(), &mint_msg, &[])
            .unwrap();
    }

    mod create_listing_tests {
        use super::*;

        #[test]
        fn test_mint_nft() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let asset_contract = setup_asset_contract(&mut app, &minter);

            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");
        }

        #[test]
        fn test_create_listing_success() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Approve marketplace contract to manage the NFT
            let approve_msg = Cw721ExecuteMsg::Approve {
                spender: marketplace_contract.to_string(),
                token_id: "token1".to_string(),
                expires: None,
            };
            app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
                .unwrap();

            // Create listing
            let price = coin(100, "uxion");
            let list_msg = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price: price.clone(),
                token_id: "token1".to_string(),
            };

            let result =
                app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
            assert!(result.is_ok());
            let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
            assert!(listing_resp.is_ok());
            let listing = listing_resp.unwrap();
            assert_eq!(listing.price, price);
            // TODO: seller should not be the marketplace contract
            // assert_eq!(listing.seller, seller);

            // verify event is emitted
            let events = result.unwrap().events;
            let list_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item");
            assert!(list_event.is_some());
            let listing_id = generate_id(vec![&asset_contract.as_bytes(), &"token1".as_bytes()]);
            assert_eq!(
                "54f6c0b51fa2fae79401bc3d0f0e5d98f8be4588d312643f7b7dd631e88173cc",
                listing_id.clone()
            );
            let listing = app
                .wrap()
                .query_wasm_smart::<Listing>(
                    marketplace_contract.clone(),
                    &QueryMsg::Listing {
                        listing_id: listing_id,
                    },
                )
                .unwrap();

            assert_eq!(listing.price, price);
            assert_eq!(listing.seller, seller);
            assert_eq!(listing.status, ListingStatus::Active);
        }
    }
}

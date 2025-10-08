use cosmwasm_std::{coin, Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper};
use serde_json::json;
use xion_nft_marketplace::msg::{ExecuteMsg, InstantiateMsg};

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
        xion_nft_marketplace::contract::query,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset::msg::InstantiateMsg as AssetInstantiateMsg;
    use cw721::DefaultOptionalCollectionExtensionMsg;
    use cw_multi_test::{App, Executor};

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
            // If we get here without panic, minting worked
        }

        #[test]
        fn test_marketplace_list_item_without_asset() {
            let mut app = setup_app();
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to create a listing with a non-existent asset contract
            let price = coin(100, "uxion");
            let list_msg = ExecuteMsg::ListItem {
                collection: "cosmwasm1nonexistent".to_string(),
                price,
                token_id: "token1".to_string(),
            };

            let result =
                app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);

            // This should fail because the asset contract doesn't exist
            assert!(result.is_err());
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
            let approve_msg = asset::msg::ExecuteMsg::<
                cw721::DefaultOptionalNftExtensionMsg,
                cw721::DefaultOptionalCollectionExtensionMsg,
                asset::msg::AssetExtensionExecuteMsg,
            >::Approve {
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

            if let Err(e) = &result {
                println!("Error: {}", e);
            }
            assert!(result.is_ok());

            // Verify the listing was created by checking events
            let events = result.unwrap().events;
            let list_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item");
            assert!(list_event.is_some());

            let list_event = list_event.unwrap();
            // Find attributes by key since order might vary
            let id_attr = list_event
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap();
            let owner_attr = list_event
                .attributes
                .iter()
                .find(|a| a.key == "owner")
                .unwrap();
            let collection_attr = list_event
                .attributes
                .iter()
                .find(|a| a.key == "collection")
                .unwrap();
            let token_id_attr = list_event
                .attributes
                .iter()
                .find(|a| a.key == "token_id")
                .unwrap();
            let price_attr = list_event
                .attributes
                .iter()
                .find(|a| a.key == "price")
                .unwrap();

            assert_eq!(token_id_attr.value, "token1");
            assert_eq!(owner_attr.value, seller.to_string());
            assert_eq!(collection_attr.value, asset_contract.to_string());
            assert_eq!(price_attr.value, price.to_string());
        }

        #[test]
        fn test_create_listing_unauthorized() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let unauthorized_user = app.api().addr_make("unauthorized");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Try to create listing with unauthorized user
            let price = coin(100, "uxion");
            let list_msg = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price,
                token_id: "token1".to_string(),
            };

            let result = app.execute_contract(
                unauthorized_user.clone(),
                marketplace_contract.clone(),
                &list_msg,
                &[],
            );

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("sender is not owner"));
        }

        #[test]
        fn test_create_listing_invalid_denom() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Try to create listing with invalid denom
            let price = coin(100, "uatom"); // Wrong denom
            let list_msg = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price,
                token_id: "token1".to_string(),
            };

            let result =
                app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Invalid listing denom"));
        }

        #[test]
        fn test_create_listing_already_listed() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create first listing
            let price = coin(100, "uxion");
            let list_msg = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price: price.clone(),
                token_id: "token1".to_string(),
            };

            let result =
                app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
            assert!(result.is_ok());

            // Try to create second listing for same token
            let list_msg2 = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price,
                token_id: "token1".to_string(),
            };

            let result2 = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &list_msg2,
                &[],
            );

            assert!(result2.is_err());
            let error = result2.unwrap_err();
            assert!(error.to_string().contains("Already listed"));
        }

        #[test]
        fn test_create_listing_nonexistent_token() {
            let mut app = setup_app();
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");
            let minter = app.api().addr_make("minter");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to create listing for non-existent token
            let price = coin(100, "uxion");
            let list_msg = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price,
                token_id: "nonexistent".to_string(),
            };

            let result =
                app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("sender is not owner"));
        }
    }

    mod cancel_listing_tests {
        use super::*;

        fn create_listing(
            app: &mut App,
            marketplace_contract: &Addr,
            asset_contract: &Addr,
            seller: &Addr,
            token_id: &str,
            price: cosmwasm_std::Coin,
        ) -> String {
            // Approve marketplace contract to manage the NFT
            let approve_msg = asset::msg::ExecuteMsg::<
                cw721::DefaultOptionalNftExtensionMsg,
                cw721::DefaultOptionalCollectionExtensionMsg,
                asset::msg::AssetExtensionExecuteMsg,
            >::Approve {
                spender: marketplace_contract.to_string(),
                token_id: token_id.to_string(),
                expires: None,
            };
            app.execute_contract(seller.clone(), asset_contract.clone(), &approve_msg, &[])
                .unwrap();

            let list_msg = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price,
                token_id: token_id.to_string(),
            };

            let result =
                app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);
            assert!(result.is_ok());

            // Extract listing ID from events
            let events = result.unwrap().events;
            let list_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
                .unwrap();
            // Find the id attribute by key
            let id_attr = list_event
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap();
            id_attr.value.clone()
        }

        #[test]
        fn test_cancel_listing_success() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create listing
            let price = coin(100, "uxion");
            let listing_id = create_listing(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                price,
            );

            // Cancel listing
            let cancel_msg = ExecuteMsg::CancelListing {
                listing_id: listing_id.clone(),
            };

            let result = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_ok());

            // Verify the listing was cancelled by checking events
            let events = result.unwrap().events;
            let cancel_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/cancel-listing");
            assert!(cancel_event.is_some());

            let cancel_event = cancel_event.unwrap();
            // Find attributes by key since order might vary
            let id_attr = cancel_event
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap();
            let owner_attr = cancel_event
                .attributes
                .iter()
                .find(|a| a.key == "owner")
                .unwrap();
            let collection_attr = cancel_event
                .attributes
                .iter()
                .find(|a| a.key == "collection")
                .unwrap();
            let token_id_attr = cancel_event
                .attributes
                .iter()
                .find(|a| a.key == "token_id")
                .unwrap();

            assert_eq!(id_attr.value, listing_id);
            assert_eq!(owner_attr.value, seller.to_string());
            assert_eq!(collection_attr.value, asset_contract.to_string());
            assert_eq!(token_id_attr.value, "token1");
        }

        #[test]
        fn test_cancel_listing_unauthorized() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let unauthorized_user = app.api().addr_make("unauthorized");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create listing
            let price = coin(100, "uxion");
            let listing_id = create_listing(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                price,
            );

            // Try to cancel listing with unauthorized user
            let cancel_msg = ExecuteMsg::CancelListing { listing_id };

            let result = app.execute_contract(
                unauthorized_user.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("sender is not the seller"));
        }

        #[test]
        fn test_cancel_listing_nonexistent() {
            let mut app = setup_app();
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to cancel non-existent listing
            let cancel_msg = ExecuteMsg::CancelListing {
                listing_id: "nonexistent".to_string(),
            };

            let result = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_err());
            let error = result.unwrap_err();
            assert!(error.to_string().contains("Listing not found"));
        }

        #[test]
        fn test_cancel_listing_with_sale_approvals() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup marketplace with sale approvals enabled
            let marketplace_code_id = app.store_code(marketplace_contract());
            let config_json = json!({
                "manager": manager.to_string(),
                "sale_approvals": true,
                "fee_bps": 250,
                "listing_denom": "uxion"
            });

            let instantiate_msg = InstantiateMsg {
                config: serde_json::from_value(config_json).unwrap(),
            };
            let marketplace_contract = app
                .instantiate_contract(
                    marketplace_code_id,
                    manager.clone(),
                    &instantiate_msg,
                    &[],
                    "test-marketplace",
                    None,
                )
                .unwrap();

            // Setup asset contract
            let asset_contract = setup_asset_contract(&mut app, &minter);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create listing
            let price = coin(100, "uxion");
            let listing_id = create_listing(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                price,
            );

            // Note: In a real scenario, we would need to simulate the listing status
            // changing to PendingApproval, but for this test we'll assume the listing
            // remains Active and can be cancelled normally
            let cancel_msg = ExecuteMsg::CancelListing { listing_id };

            let result = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            // This should succeed because the listing is still Active
            assert!(result.is_ok());
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn test_list_and_cancel_workflow() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

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

            // Extract listing ID from events
            let events = result.unwrap().events;
            let list_event = events
                .iter()
                .find(|e| e.ty == "marketplace/list-item")
                .unwrap();
            let listing_id = list_event.attributes[0].value.clone();

            // Cancel listing
            let cancel_msg = ExecuteMsg::CancelListing {
                listing_id: listing_id.clone(),
            };

            let result = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );
            assert!(result.is_ok());

            // Verify both events occurred
            let events = result.unwrap().events;
            let cancel_event = events.iter().find(|e| e.ty == "marketplace/cancel-listing");
            assert!(cancel_event.is_some());
        }

        #[test]
        fn test_multiple_listings_different_tokens() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint multiple NFTs to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token2");

            // Create listings for both tokens
            let price1 = coin(100, "uxion");
            let price2 = coin(200, "uxion");

            let list_msg1 = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price: price1,
                token_id: "token1".to_string(),
            };

            let list_msg2 = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price: price2,
                token_id: "token2".to_string(),
            };

            let result1 = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &list_msg1,
                &[],
            );
            assert!(result1.is_ok());

            let result2 = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &list_msg2,
                &[],
            );
            assert!(result2.is_ok());

            // Both listings should be created successfully
            let events1 = result1.unwrap().events;
            let events2 = result2.unwrap().events;

            let list_event1 = events1.iter().find(|e| e.ty == "marketplace/list-item");
            let list_event2 = events2.iter().find(|e| e.ty == "marketplace/list-item");

            assert!(list_event1.is_some());
            assert!(list_event2.is_some());
        }
    }
}

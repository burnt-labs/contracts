use anyhow::Error;
use cosmwasm_std::{coin, Addr, Empty};
use cw_multi_test::AppResponse;
use cw_multi_test::{Contract, ContractWrapper};
use serde_json::json;
use xion_nft_marketplace::helpers::generate_id;
use xion_nft_marketplace::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use xion_nft_marketplace::state::{Listing, ListingStatus};

pub fn assert_error(result: Result<AppResponse, Error>, expected: String) {
    assert_eq!(result.unwrap_err().source().unwrap().to_string(), expected);
}
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

    fn setup_app_with_balances() -> App {
        use cosmwasm_std::coin;
        use cw_multi_test::{App, BankSudo, SudoMsg};

        let mut app = App::default();

        // Create proper addresses using app.api().addr_make
        let buyer = app.api().addr_make("buyer");
        let seller = app.api().addr_make("seller");
        let minter = app.api().addr_make("minter");
        let manager = app.api().addr_make("manager");

        // Mint tokens to test accounts using sudo
        let funds = vec![coin(10000, "uxion")];

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: buyer.to_string(),
            amount: funds.clone(),
        }))
        .unwrap();

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: seller.to_string(),
            amount: funds.clone(),
        }))
        .unwrap();

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: minter.to_string(),
            amount: funds.clone(),
        }))
        .unwrap();

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: manager.to_string(),
            amount: funds.clone(),
        }))
        .unwrap();

        app
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
            "fee_recipient": manager.to_string(),
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
            assert_eq!(listing.seller, seller);

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
                    &QueryMsg::Listing { listing_id },
                )
                .unwrap();

            assert_eq!(listing.price, price);
            assert_eq!(listing.seller, seller);
            assert_eq!(listing.status, ListingStatus::Active);
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
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::Unauthorized {
                    message: "sender is not owner".to_string(),
                }
                .to_string(),
            );
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
            let price = coin(100, "fakexion"); // Wrong denom
            let list_msg = ExecuteMsg::ListItem {
                collection: asset_contract.to_string(),
                price,
                token_id: "token1".to_string(),
            };

            let result =
                app.execute_contract(seller.clone(), marketplace_contract.clone(), &list_msg, &[]);

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidListingDenom {
                    expected: "uxion".to_string(),
                    actual: "fakexion".to_string(),
                }
                .to_string(),
            );
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
            assert_error(
                result2,
                xion_nft_marketplace::error::ContractError::AlreadyListed {}.to_string(),
            );
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
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::Unauthorized {
                    message: "sender is not owner".to_string(),
                }
                .to_string(),
            );
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
            let approve_msg = Cw721ExecuteMsg::Approve {
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

            // listing should not be found on the asset contract
            let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
            assert!(listing_resp.is_err());

            // listing should not be found on the marketplace contract
            let listing_resp = app.wrap().query_wasm_smart::<Listing>(
                marketplace_contract.clone(),
                &QueryMsg::Listing {
                    listing_id: listing_id.clone(),
                },
            );
            assert!(listing_resp.is_err());
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

            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::Unauthorized {
                    message: "sender is not the seller".to_string(),
                }
                .to_string(),
            );
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
            result.unwrap_err().to_string().contains("not found");
        }
    }

    mod buy_item_tests {
        use super::*;

        fn extract_listing_id_from_events(events: &[cosmwasm_std::Event]) -> String {
            let list_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
                .unwrap();
            list_event
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap()
                .value
                .clone()
        }

        fn create_listing_for_buy_test(
            app: &mut App,
            marketplace_contract: &Addr,
            asset_contract: &Addr,
            seller: &Addr,
            token_id: &str,
            price: cosmwasm_std::Coin,
        ) -> String {
            // Approve marketplace contract to manage the NFT
            let approve_msg = Cw721ExecuteMsg::Approve {
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
            let id_attr = list_event
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap();
            id_attr.value.clone()
        }

        #[test]
        fn test_buy_item_success() {
            // This test follows the exact same pattern as test_create_listing_success
            // but adds the buy functionality
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
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

            // Create listing (same as test_create_listing_success)
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
            let listing_id = extract_listing_id_from_events(&events);

            // Verify listing was created (same as test_create_listing_success)
            let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
            assert!(listing_resp.is_ok());
            let listing = listing_resp.unwrap();
            assert_eq!(listing.price, price);

            // Add funds for the buyer
            use cosmwasm_std::coin;
            use cw_multi_test::{BankSudo, SudoMsg};
            let funds = vec![coin(10000, "uxion")];
            app.sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: buyer.to_string(),
                amount: funds,
            }))
            .unwrap();

            // Buy item using listing_id
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                std::slice::from_ref(&price),
            );

            match result {
                Ok(response) => {
                    // Verify the item was sold by checking events
                    let events = response.events;
                    let sell_event = events
                        .iter()
                        .find(|e| e.ty == "wasm-xion-nft-marketplace/item-sold");
                    assert!(sell_event.is_some());

                    // Verify NFT ownership changed
                    let owner_query = cw721_base::msg::QueryMsg::OwnerOf {
                        token_id: "token1".to_string(),
                        include_expired: Some(false),
                    };
                    let owner_resp: cw721::msg::OwnerOfResponse = app
                        .wrap()
                        .query_wasm_smart(asset_contract.clone(), &owner_query)
                        .unwrap();
                    assert_eq!(owner_resp.owner, buyer.to_string());

                    // Verify listing is no longer active
                    let listing_resp = query_listing(&app.wrap(), &asset_contract, "token1");
                    assert!(listing_resp.is_err());
                }
                Err(error) => {
                    println!("Error: {:?}", error);
                    panic!("Buy item failed: {:?}", error);
                }
            }
        }

        #[test]
        fn test_buy_item_insufficient_funds() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create listing
            let price = coin(100, "uxion");
            let listing_id = create_listing_for_buy_test(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                price.clone(),
            );

            // Try to buy item with insufficient funds
            let insufficient_price = coin(50, "uxion");
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                &[insufficient_price],
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidPayment {
                    expected: price,
                    actual: coin(50, "uxion"),
                }
                .to_string(),
            );
        }

        #[test]
        fn test_buy_item_wrong_price() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create listing
            let listing_price = coin(100, "uxion");
            let listing_id = create_listing_for_buy_test(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                listing_price.clone(),
            );

            // Try to buy item with wrong price
            let wrong_price = coin(150, "uxion");
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: wrong_price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                &[wrong_price],
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidPrice {
                    expected: listing_price,
                    actual: coin(150, "uxion"),
                }
                .to_string(),
            );
        }

        #[test]
        fn test_buy_item_wrong_denomination() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
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

            // Extract listing ID from events
            let events = result.unwrap().events;
            let listing_id = extract_listing_id_from_events(&events);

            // Add funds for the buyer
            use cosmwasm_std::coin;
            use cw_multi_test::{BankSudo, SudoMsg};
            let funds = vec![coin(10000, "fakexion")]; // Give buyer fakexion instead of uxion
            app.sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: buyer.to_string(),
                amount: funds,
            }))
            .unwrap();

            // Try to buy item with wrong denomination
            let wrong_denom_price = coin(100, "fakexion");
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: wrong_denom_price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                &[wrong_denom_price],
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidPrice {
                    expected: coin(100, "uxion"),
                    actual: coin(100, "fakexion"),
                }
                .to_string(),
            );
        }

        #[test]
        fn test_buy_item_nonexistent_listing() {
            let mut app = setup_app_with_balances();
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts

            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to buy item that's not listed
            let price = coin(100, "uxion");
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: "nonexistent-listing-id".to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                &[price],
            );

            assert!(result.is_err());
            // Should fail because the listing doesn't exist
            let error_msg = result.unwrap_err().to_string();
            // The error is wrapped by the test framework, so we check for the generic error
            assert!(error_msg.contains("Error executing") || error_msg.contains("WasmMsg"));
        }

        #[test]
        fn test_buy_item_self_purchase() {
            let mut app = setup_app_with_balances();
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
            let listing_id = create_listing_for_buy_test(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                price.clone(),
            );

            // Try to buy own item (this should succeed - sellers can buy their own items)
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                seller.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                &[price],
            );

            // This should succeed - sellers can buy their own items
            assert!(result.is_ok());

            // Verify the item was sold by checking events
            let events = result.unwrap().events;
            let sell_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/item-sold");
            assert!(sell_event.is_some());
        }

        #[test]
        fn test_buy_item_multiple_coins() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create listing
            let price = coin(100, "uxion");
            let listing_id = create_listing_for_buy_test(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                price.clone(),
            );

            // Try to buy item with multiple coins
            let extra_coin = coin(50, "fakexion");
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                &[price, extra_coin],
            );

            assert!(result.is_err());
            // Should fail because only one coin is expected
            let error_msg = result.unwrap_err().to_string();
            // The error is wrapped by the test framework, so we check for the generic error
            assert!(error_msg.contains("Error executing") || error_msg.contains("WasmMsg"));
        }

        #[test]
        fn test_buy_item_no_coins() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create listing
            let price = coin(100, "uxion");
            let listing_id = create_listing_for_buy_test(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &seller,
                "token1",
                price.clone(),
            );

            // Try to buy item without sending any coins
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: price.clone(),
            };

            let result =
                app.execute_contract(buyer.clone(), marketplace_contract.clone(), &buy_msg, &[]);

            assert!(result.is_err());
            // Should fail because no coins were sent
            let error_msg = result.unwrap_err().to_string();
            // The error is wrapped by the test framework, so we check for the generic error
            assert!(error_msg.contains("Error executing") || error_msg.contains("WasmMsg"));
        }

        #[test]
        fn test_buy_item_success_with_royalties() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");
            let royalty_recipient = app.api().addr_make("royalty_recipient");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Set up royalty plugin on the collection (5% = 500 bps)
            let royalty_plugin = asset::plugin::Plugin::Royalty {
                bps: 500, // 5%
                recipient: royalty_recipient.clone(),
                on_primary: true, // Collect royalties on all sales including primary
            };

            let set_plugin_msg = asset::msg::ExecuteMsg::<
                cw721::DefaultOptionalNftExtensionMsg,
                cw721::DefaultOptionalCollectionExtensionMsg,
                asset::msg::AssetExtensionExecuteMsg,
            >::UpdateExtension {
                msg: asset::msg::AssetExtensionExecuteMsg::SetCollectionPlugin {
                    plugins: vec![royalty_plugin],
                },
            };

            app.execute_contract(minter.clone(), asset_contract.clone(), &set_plugin_msg, &[])
                .unwrap();

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

            // Create listing with price of 1000 uxion
            let price = coin(1000, "uxion");
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
            let listing_id = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/list-item")
                .unwrap()
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap()
                .value
                .clone();

            // Get initial balances
            let seller_balance_before = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
            let buyer_balance_before = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
            let manager_balance_before =
                app.wrap().query_balance(&manager, "uxion").unwrap().amount;
            let royalty_recipient_balance_before = app
                .wrap()
                .query_balance(&royalty_recipient, "uxion")
                .unwrap()
                .amount;

            // Buy item
            let buy_msg = ExecuteMsg::BuyItem {
                listing_id: listing_id.clone(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &buy_msg,
                std::slice::from_ref(&price),
            );

            assert!(result.is_ok());

            // Get final balances
            let seller_balance_after = app.wrap().query_balance(&seller, "uxion").unwrap().amount;
            let buyer_balance_after = app.wrap().query_balance(&buyer, "uxion").unwrap().amount;
            let manager_balance_after = app.wrap().query_balance(&manager, "uxion").unwrap().amount;
            let royalty_recipient_balance_after = app
                .wrap()
                .query_balance(&royalty_recipient, "uxion")
                .unwrap()
                .amount;

            // Calculate expected amounts
            // Price: 1000 uxion
            // Marketplace fee (2.5% = 250 bps): 1000 * 250 / 10000 = 25 uxion
            // Royalty (5% = 500 bps): 1000 * 500 / 10000 = 50 uxion
            // Seller gets: 1000 - 25 - 50 = 925 uxion
            let expected_marketplace_fee = 25u128;
            let expected_royalty = 50u128;
            let expected_seller_payment = 925u128;

            // Verify payment distribution
            use cosmwasm_std::Uint128;

            assert_eq!(
                buyer_balance_after,
                buyer_balance_before - Uint128::from(1000u128),
                "Buyer should have paid 1000 uxion"
            );

            assert_eq!(
                seller_balance_after,
                seller_balance_before + Uint128::from(expected_seller_payment),
                "Seller should receive {} uxion (1000 - 25 marketplace fee - 50 royalty)",
                expected_seller_payment
            );

            assert_eq!(
                manager_balance_after,
                manager_balance_before + Uint128::from(expected_marketplace_fee),
                "Manager should receive {} uxion marketplace fee",
                expected_marketplace_fee
            );

            assert_eq!(
                royalty_recipient_balance_after,
                royalty_recipient_balance_before + Uint128::from(expected_royalty),
                "Royalty recipient should receive {} uxion royalty",
                expected_royalty
            );

            // Verify NFT ownership changed
            let owner_query = cw721_base::msg::QueryMsg::OwnerOf {
                token_id: "token1".to_string(),
                include_expired: Some(false),
            };
            let owner_resp: cw721::msg::OwnerOfResponse = app
                .wrap()
                .query_wasm_smart(asset_contract.clone(), &owner_query)
                .unwrap();
            assert_eq!(owner_resp.owner, buyer.to_string());
        }
    }

    mod create_offer_tests {
        use super::*;

        #[test]
        fn test_create_offer_success() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create offer
            let price = coin(100, "uxion");
            let offer_msg = ExecuteMsg::CreateOffer {
                collection: asset_contract.to_string(),
                token_id: "token1".to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                std::slice::from_ref(&price),
            );

            assert!(result.is_ok());

            // Verify the offer was created by checking event
            let response = result.unwrap();
            let events = response.events;
            let offer_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer");
            assert!(offer_event.is_some());

            // Verify event has ID attribute
            let id_attr = offer_event
                .unwrap()
                .attributes
                .iter()
                .find(|a| a.key == "id");
            assert!(id_attr.is_some());
        }

        #[test]
        fn test_create_offer_without_funds() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Try to create offer without sending funds
            let price = coin(100, "uxion");
            let offer_msg = ExecuteMsg::CreateOffer {
                collection: asset_contract.to_string(),
                token_id: "token1".to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                &[], // No funds sent
            );

            assert!(result.is_err());
            // Should fail because no funds were sent for escrow
        }

        #[test]
        fn test_create_offer_wrong_denom() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Mint wrong denom to buyer
            use cw_multi_test::{BankSudo, SudoMsg};
            let funds = vec![coin(10000, "fakexion")];
            app.sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: buyer.to_string(),
                amount: funds,
            }))
            .unwrap();

            // Try to create offer with wrong denom
            let price = coin(100, "fakexion");
            let offer_msg = ExecuteMsg::CreateOffer {
                collection: asset_contract.to_string(),
                token_id: "token1".to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                std::slice::from_ref(&price),
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidListingDenom {
                    expected: "uxion".to_string(),
                    actual: "fakexion".to_string(),
                }
                .to_string(),
            );
        }

        #[test]
        fn test_create_offer_insufficient_funds() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Try to create offer with insufficient funds
            let price = coin(100, "uxion");
            let insufficient = coin(50, "uxion");
            let offer_msg = ExecuteMsg::CreateOffer {
                collection: asset_contract.to_string(),
                token_id: "token1".to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                std::slice::from_ref(&insufficient),
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidPayment {
                    expected: price,
                    actual: insufficient,
                }
                .to_string(),
            );
        }

        #[test]
        fn test_create_offer_nonexistent_token() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Create offer for nonexistent token (should still succeed as offers don't validate existence)
            let price = coin(100, "uxion");
            let offer_msg = ExecuteMsg::CreateOffer {
                collection: asset_contract.to_string(),
                token_id: "nonexistent".to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                std::slice::from_ref(&price),
            );

            // Offers can be created for nonexistent tokens
            assert!(result.is_ok());
        }
    }

    mod cancel_offer_tests {
        use super::*;

        fn create_offer_helper(
            app: &mut App,
            marketplace_contract: &Addr,
            asset_contract: &Addr,
            buyer: &Addr,
            token_id: &str,
            price: cosmwasm_std::Coin,
        ) -> String {
            let offer_msg = ExecuteMsg::CreateOffer {
                collection: asset_contract.to_string(),
                token_id: token_id.to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                &[price],
            );
            assert!(result.is_ok());

            // Extract offer ID from events
            let events = result.unwrap().events;
            let offer_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/create-offer")
                .unwrap();
            offer_event
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap()
                .value
                .clone()
        }

        #[test]
        fn test_cancel_offer_success() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Check buyer balance before
            let buyer_balance_before = app.wrap().query_balance(buyer.clone(), "uxion").unwrap();

            // Create offer and get ID from event
            let price = coin(100, "uxion");
            let offer_id = create_offer_helper(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &buyer,
                "token1",
                price.clone(),
            );

            // Cancel offer
            let cancel_msg = ExecuteMsg::CancelOffer { id: offer_id };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_ok());

            // Verify refund was sent
            let buyer_balance_after = app.wrap().query_balance(buyer.clone(), "uxion").unwrap();
            assert_eq!(buyer_balance_before, buyer_balance_after);

            // Verify event was emitted
            let events = result.unwrap().events;
            let cancel_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/cancel-offer");
            assert!(cancel_event.is_some());
        }

        #[test]
        fn test_cancel_offer_unauthorized() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let seller = app.api().addr_make("seller");
            let buyer = app.api().addr_make("buyer");
            let unauthorized = app.api().addr_make("unauthorized");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint NFT to seller
            mint_nft(&mut app, &asset_contract, &minter, &seller, "token1");

            // Create offer and get ID from event
            let price = coin(100, "uxion");
            let offer_id = create_offer_helper(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &buyer,
                "token1",
                price.clone(),
            );

            // Try to cancel offer with unauthorized user
            let cancel_msg = ExecuteMsg::CancelOffer { id: offer_id };

            let result = app.execute_contract(
                unauthorized.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::Unauthorized {
                    message: "sender is not the buyer".to_string(),
                }
                .to_string(),
            );
        }

        #[test]
        fn test_cancel_offer_nonexistent() {
            let mut app = setup_app_with_balances();
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");
            let minter = app.api().addr_make("minter");

            // Setup contracts
            let _asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to cancel nonexistent offer
            let cancel_msg = ExecuteMsg::CancelOffer {
                id: "nonexistent-offer-id".to_string(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_err());
            // Should fail because offer doesn't exist
        }
    }

    mod create_collection_offer_tests {
        use super::*;

        #[test]
        fn test_create_collection_offer_success() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Create collection offer
            let price = coin(100, "uxion");
            let offer_msg = ExecuteMsg::CreateCollectionOffer {
                collection: asset_contract.to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                std::slice::from_ref(&price),
            );

            assert!(result.is_ok());
        }

        #[test]
        fn test_create_collection_offer_without_funds() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to create collection offer without funds
            let price = coin(100, "uxion");
            let offer_msg = ExecuteMsg::CreateCollectionOffer {
                collection: asset_contract.to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                &[], // No funds
            );

            assert!(result.is_err());
        }

        #[test]
        fn test_create_collection_offer_wrong_denom() {
            let mut app = setup_app();
            let minter = app.api().addr_make("minter");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Mint wrong denom to buyer
            use cw_multi_test::{BankSudo, SudoMsg};
            let funds = vec![coin(10000, "fakexion")];
            app.sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: buyer.to_string(),
                amount: funds,
            }))
            .unwrap();

            // Try to create collection offer with wrong denom
            let price = coin(100, "fakexion");
            let offer_msg = ExecuteMsg::CreateCollectionOffer {
                collection: asset_contract.to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                std::slice::from_ref(&price),
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidListingDenom {
                    expected: "uxion".to_string(),
                    actual: "fakexion".to_string(),
                }
                .to_string(),
            );
        }

        #[test]
        fn test_create_collection_offer_insufficient_funds() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to create collection offer with insufficient funds
            let price = coin(100, "uxion");
            let insufficient = coin(50, "uxion");
            let offer_msg = ExecuteMsg::CreateCollectionOffer {
                collection: asset_contract.to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                std::slice::from_ref(&insufficient),
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::InvalidPayment {
                    expected: price,
                    actual: insufficient,
                }
                .to_string(),
            );
        }
    }

    mod cancel_collection_offer_tests {
        use super::*;

        fn create_collection_offer_helper(
            app: &mut App,
            marketplace_contract: &Addr,
            asset_contract: &Addr,
            buyer: &Addr,
            price: cosmwasm_std::Coin,
        ) -> String {
            let offer_msg = ExecuteMsg::CreateCollectionOffer {
                collection: asset_contract.to_string(),
                price: price.clone(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &offer_msg,
                &[price],
            );
            assert!(result.is_ok());

            // Extract collection offer ID from events
            let events = result.unwrap().events;
            let offer_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/create-collection-offer")
                .unwrap();
            offer_event
                .attributes
                .iter()
                .find(|a| a.key == "id")
                .unwrap()
                .value
                .clone()
        }

        #[test]
        fn test_cancel_collection_offer_success() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Check buyer balance before
            let buyer_balance_before = app.wrap().query_balance(buyer.clone(), "uxion").unwrap();

            // Create collection offer and get ID from event
            let price = coin(100, "uxion");
            let offer_id = create_collection_offer_helper(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &buyer,
                price.clone(),
            );

            // Cancel collection offer
            let cancel_msg = ExecuteMsg::CancelCollectionOffer { id: offer_id };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_ok());

            // Verify refund was sent
            let buyer_balance_after = app.wrap().query_balance(buyer.clone(), "uxion").unwrap();
            assert_eq!(buyer_balance_before, buyer_balance_after);

            // Verify event was emitted
            let events = result.unwrap().events;
            let cancel_event = events
                .iter()
                .find(|e| e.ty == "wasm-xion-nft-marketplace/cancel-collection-offer");
            assert!(cancel_event.is_some());
        }

        #[test]
        fn test_cancel_collection_offer_unauthorized() {
            let mut app = setup_app_with_balances();
            let minter = app.api().addr_make("minter");
            let buyer = app.api().addr_make("buyer");
            let unauthorized = app.api().addr_make("unauthorized");
            let manager = app.api().addr_make("manager");

            // Setup contracts
            let asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Create collection offer and get ID from event
            let price = coin(100, "uxion");
            let offer_id = create_collection_offer_helper(
                &mut app,
                &marketplace_contract,
                &asset_contract,
                &buyer,
                price.clone(),
            );

            // Try to cancel with unauthorized user
            let cancel_msg = ExecuteMsg::CancelCollectionOffer { id: offer_id };

            let result = app.execute_contract(
                unauthorized.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_err());
            assert_error(
                result,
                xion_nft_marketplace::error::ContractError::Unauthorized {
                    message: "sender is not the buyer".to_string(),
                }
                .to_string(),
            );
        }

        #[test]
        fn test_cancel_collection_offer_nonexistent() {
            let mut app = setup_app_with_balances();
            let buyer = app.api().addr_make("buyer");
            let manager = app.api().addr_make("manager");
            let minter = app.api().addr_make("minter");

            // Setup contracts
            let _asset_contract = setup_asset_contract(&mut app, &minter);
            let marketplace_contract = setup_marketplace_contract(&mut app, &manager);

            // Try to cancel nonexistent collection offer
            let cancel_msg = ExecuteMsg::CancelCollectionOffer {
                id: "nonexistent-collection-offer".to_string(),
            };

            let result = app.execute_contract(
                buyer.clone(),
                marketplace_contract.clone(),
                &cancel_msg,
                &[],
            );

            assert!(result.is_err());
        }
    }
}

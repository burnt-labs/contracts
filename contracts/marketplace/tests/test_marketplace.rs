use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

fn asset_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        asset::contracts::asset_base::execute,
        asset::contracts::asset_base::instantiate,
        asset::contracts::asset_base::query,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset::msg::InstantiateMsg;
    use cw721::DefaultOptionalCollectionExtensionMsg;
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_asset_contract_instantiation() {
        let mut app = App::default();

        // Store the asset contract
        let asset_code_id = app.store_code(asset_contract());

        // Instantiate the asset contract
        let minter = app.api().addr_make("minter");
        let instantiate_msg = InstantiateMsg {
            name: "Test Asset".to_string(),
            symbol: "TEST".to_string(),
            minter: Some(minter.to_string()),
            collection_info_extension: DefaultOptionalCollectionExtensionMsg::default(),
            creator: Some(minter.to_string()),
            withdraw_address: None,
        };

        let asset_addr = app
            .instantiate_contract(
                asset_code_id,
                minter.clone(),
                &instantiate_msg,
                &[],
                "test-asset",
                None,
            )
            .unwrap();

        // Verify the contract was instantiated successfully
        assert!(!asset_addr.to_string().is_empty());
    }

    #[test]
    fn test_asset_contract_creation() {
        let _contract = asset_contract();
        // Verify we can create the contract wrapper without errors
    }
}

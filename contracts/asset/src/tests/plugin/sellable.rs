use std::collections::HashMap;

use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, Empty,
    testing::{message_info, mock_dependencies, mock_env},
};
use cw721::{msg::Cw721ExecuteMsg, state::NftInfo};

use crate::{
    msg::AssetExtensionExecuteMsg,
    plugin::Plugin,
    state::{AssetConfig, ListingInfo},
    traits::{DefaultAssetContract, PluggableAsset},
};

#[test]
fn buy_deducts_royalty_fees() {
    let mut deps = mock_dependencies();
    let contract: DefaultAssetContract<'static, Empty, Empty, Empty, Empty> = Default::default();

    let seller = deps.api.addr_make("seller");
    let buyer = deps.api.addr_make("buyer");
    let royalty_recipient = deps.api.addr_make("artist");

    let price = Coin::new(1_000u128, "uxion");

    let nft_info = NftInfo {
        owner: seller.clone(),
        approvals: vec![],
        token_uri: None,
        extension: Empty::default(),
    };
    let listing = ListingInfo {
        id: "token-1".to_string(),
        price: price.clone(),
        seller: seller.clone(),
        reserved: None,
    };

    contract
        .config
        .listings
        .save(deps.as_mut().storage, "token-1", &listing)
        .unwrap();
    contract
        .config
        .cw721_config
        .nft_info
        .save(deps.as_mut().storage, "token-1", &nft_info)
        .unwrap();
    contract
        .config
        .collection_plugins
        .save(
            deps.as_mut().storage,
            "Royalty",
            &Plugin::Royalty {
                bps: 500,
                recipient: royalty_recipient.clone(),
            },
        )
        .unwrap();

    let env = mock_env();
    let info = message_info(&buyer, std::slice::from_ref(&price));

    let res = contract
        .execute_pluggable(
            deps.as_mut(),
            &env,
            &info,
            Cw721ExecuteMsg::UpdateExtension {
                msg: AssetExtensionExecuteMsg::Buy {
                    token_id: "token-1".to_string(),
                    recipient: None,
                },
            },
        )
        .unwrap();

    assert_eq!(res.messages.len(), 2);

    let mut seller_paid = None;
    let mut royalty_paid = None;

    for msg in &res.messages {
        match &msg.msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                let coin = amount
                    .first()
                    .cloned()
                    .expect("send message must include funds");
                if *to_address == seller.to_string() {
                    seller_paid = Some(coin);
                } else if *to_address == royalty_recipient.to_string() {
                    royalty_paid = Some(coin);
                } else {
                    panic!("unexpected recipient {}", to_address);
                }
            }
            other => panic!("unexpected message: {:?}", other),
        }
    }

    assert_eq!(
        royalty_paid.expect("royalty fee"),
        Coin::new(50u128, "uxion"),
    );
    assert_eq!(
        seller_paid.expect("seller payment"),
        Coin::new(950u128, "uxion"),
    );

    let attrs: HashMap<_, _> = res
        .attributes
        .iter()
        .map(|attr| (attr.key.clone(), attr.value.clone()))
        .collect();
    assert_eq!(
        attrs.get("royalty_amount"),
        Some(&Coin::new(50u128, "uxion").to_string()),
    );
    assert_eq!(
        attrs.get("royalty_recipient"),
        Some(&royalty_recipient.to_string()),
    );

    let stored_nft = AssetConfig::<Empty>::default()
        .cw721_config
        .nft_info
        .load(deps.as_ref().storage, "token-1")
        .unwrap();
    assert_eq!(stored_nft.owner, buyer);
}

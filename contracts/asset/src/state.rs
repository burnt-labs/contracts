use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, to_json_binary, Addr, Binary};
use cw721::{state::NftInfo, traits::{Cw721State, FromAttributesState, ToAttributesState}, NftExtension};
use cw_storage_plus::{IndexList, IndexedMap, MultiIndex};

#[cw_serde]
pub struct XionAssetCollectionMetadata {
    pub royalty_bps: Option<u16>,
    pub royalty_recipient: Option<Addr>,
    pub royalty_on_primary: Option<bool>,
    pub min_list_price: Option<u128>,
    pub not_before: Option<u64>,
    pub not_after: Option<u64>,
    pub plugins: Vec<String>,
}

impl FromAttributesState for XionAssetCollectionMetadata {
    fn from_attributes_state(attrs: &[cw721::Attribute]) -> Result<Self, cw721::error::Cw721ContractError> {
        let mut royalty_bps: Option<u16> = None;
        let mut royalty_recipient: Option<Addr> = None;
        let mut royalty_on_primary: Option<bool> = None;
        let mut min_list_price: Option<u128> = None;
        let mut not_before: Option<u64> = None;
        let mut not_after: Option<u64> = None;
        let mut plugins: Vec<String> = vec![];

        for attr in attrs {
            match attr.key.as_str() {
                "royalty_bps" => {
                    royalty_bps = Some(from_json(attr.value.clone())?);
                }
                "royalty_recipient" => {
                    royalty_recipient = Some(Addr::unchecked(from_json::<String>(attr.value.clone())?));
                }
                "royalty_on_primary" => {
                    royalty_on_primary = Some(from_json(attr.value.clone())?);
                }
                "min_list_price" => {
                    min_list_price = Some(from_json(attr.value.clone())?);
                }
                "not_before" => {
                    not_before = Some(from_json(attr.value.clone())?);
                }
                "not_after" => {
                    not_after = Some(from_json(attr.value.clone())?);
                }
                "plugins" => {
                    plugins = from_json(attr.value.clone())?;
                }
                _ => {}
            }
        }

        Ok(XionAssetCollectionMetadata {
            royalty_bps,
            royalty_recipient,
            royalty_on_primary,
            min_list_price,
            not_before,
            not_after,
            plugins,
        })
    }
}

impl ToAttributesState for XionAssetCollectionMetadata {
    fn to_attributes_state(&self) -> Result<Vec<cw721::Attribute>, cw721::error::Cw721ContractError> {
        let mut attrs: Vec<cw721::Attribute> = vec![];

        if let Some(bps) = self.royalty_bps {
            attrs.push(cw721::Attribute {
                key: "royalty_bps".to_string(),
                value: to_json_binary(&bps)?,
            });
        }

        if let Some(recipient) = &self.royalty_recipient {
            attrs.push(cw721::Attribute {
                key: "royalty_recipient".to_string(),
                value: to_json_binary(recipient)?,
            });
        }

        if let Some(on_primary) = self.royalty_on_primary {
            attrs.push(cw721::Attribute {
                key: "royalty_on_primary".to_string(),
                value: to_json_binary(&on_primary)?,
            });
        }

        if let Some(price) = self.min_list_price {
            attrs.push(cw721::Attribute {
                key: "min_list_price".to_string(),
                value: to_json_binary(&price)?,
            });
        }

        if let Some(nb) = self.not_before {
            attrs.push(cw721::Attribute {
                key: "not_before".to_string(),
                value: to_json_binary(&nb)?,
            });
        }

        if let Some(na) = self.not_after {
            attrs.push(cw721::Attribute {
                key: "not_after".to_string(),
                value: to_json_binary(&na)?,
            });
        }

        if !self.plugins.is_empty() {
            attrs.push(cw721::Attribute {
                key: "plugins".to_string(),
                value: to_json_binary(&self.plugins)?,
            });
        }

        Ok(attrs)
    }
}

impl Cw721State for XionAssetCollectionMetadata {}

#[cw_serde]
pub struct ListingInfo {
    pub id: String,
    pub price: u128,
    pub seller: Addr,
    pub is_frozen: bool,
}

pub const LISTINGS_TOKEN_INFO: IndexedMap<&str, NftInfo<NftExtension>, ListingIndexes> = IndexedMap::new("listings_token_info", ListingIndexes{
    seller: MultiIndex::new(
        |_key: &[u8], d: &NftInfo<NftExtension>| d.owner.clone(),
        "tokens",
        "listings_token_info__by_seller",
    ),
});

pub struct ListingIndexes<'a> {
    pub seller: MultiIndex<'a, Addr, NftInfo<NftExtension>, String>,
}

impl IndexList<NftInfo<NftExtension>> for ListingIndexes<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<NftInfo<NftExtension>>> + '_> {
        let v: Vec<&dyn cw_storage_plus::Index<NftInfo<NftExtension>>> = vec![&self.seller];
        Box::new(v.into_iter())
    }
}
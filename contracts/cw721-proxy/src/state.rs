use cosmwasm_std::Addr;
use cw721::extension::Cw721BaseExtensions;
use cw_storage_plus::Item;

/*
pub struct DefaultCw721ExpirationContract<'a> {
    pub expiration_days: Item<'a, u16>, // max 65535 days
    pub mint_timestamps: Map<'a, &'a str, Timestamp>,
    pub base_contract: Cw721OnchainExtensions<'a>,
}

impl Default for DefaultCw721ExpirationContract<'static> {
    fn default() -> Self {
        Self {
            expiration_days: Item::new("expiration_days"),
            mint_timestamps: Map::new("mint_timestamps"),
            base_contract: Cw721OnchainExtensions::default(),
        }
    }
}
 */

pub struct DefaultCw721ProxyContract<'a> {
    pub proxy_addr: Item<'a, Addr>,
    pub base_contract: Cw721BaseExtensions<'a>,
}

impl Default for DefaultCw721ProxyContract<'static> {
    fn default() -> Self {
        Self {
            proxy_addr: Item::new("proxy_addr"),
            base_contract: Cw721BaseExtensions::default(),
        }
    }
}

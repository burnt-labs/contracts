use cosmwasm_std::Addr;
use cw721::extension::Cw721BaseExtensions;
use cw_storage_plus::Item;

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

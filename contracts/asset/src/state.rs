use cosmwasm_std::Addr;

pub struct XionAssetCollectionMetadata {
    pub royalty_bps: Option<u16>,
    pub royalty_recipient: Option<Addr>,
    pub royalty_on_primary: Option<bool>,
    pub min_list_price: Option<u128>,
    pub not_before: Option<u64>,
    pub not_after: Option<u64>,
    pub plugins: Vec<String>,
}
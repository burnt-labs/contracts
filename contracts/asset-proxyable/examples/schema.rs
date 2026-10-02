use asset_proxyable::msg::{InstantiateMsg, MigrateMsg, ProxyableExecuteMsg, ProxyableQueryMsg};
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ProxyableExecuteMsg,
        query: ProxyableQueryMsg,
        migrate: MigrateMsg,
    }
}

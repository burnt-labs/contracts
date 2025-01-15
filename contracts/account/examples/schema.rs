use cosmwasm_schema::write_api;
use xion_account::msg::*;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg
    };
}

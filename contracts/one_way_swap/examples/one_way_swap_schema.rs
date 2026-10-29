use cosmwasm_schema::write_api;
use one_way_swap::msg::*;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg,
    };
}

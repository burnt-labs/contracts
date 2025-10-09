use cosmwasm_schema::write_api;
use opacity_verifier::msg::*;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    };
}

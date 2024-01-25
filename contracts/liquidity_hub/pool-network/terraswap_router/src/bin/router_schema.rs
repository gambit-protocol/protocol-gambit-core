use cosmwasm_schema::write_api;

use white_whale_std::pool_network::router::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        name: "terraswap-router",
        version: "1.0.0",
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg,
    }
}

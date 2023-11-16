use std::collections::HashMap;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, Contract, ContractWrapper, Executor, WasmKeeper, Router,
};
use white_whale::{
    fee::Fee,
    pool_network::{
        asset::{Asset, AssetInfo, PairType},
        pair::PoolFee,
    },
};

use super::MockAPIBech32::{MockAddressGenerator, MockApiBech32};
fn contract_pool_manager(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    ));

    app.store_code_with_creator(Addr::unchecked("admin"), contract)
}

fn store_token_code(app: &mut App<BankKeeper, MockApiBech32>) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code_with_creator(Addr::unchecked("admin"), contract)
}

#[derive(Debug)]
pub struct SuiteBuilder {
    pub cw20_balances: Vec<Cw20Coin>,
    pub native_balances: Vec<(Addr, Coin)>,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            cw20_balances: vec![],
            native_balances: vec![],
        }
    }

    pub fn with_native_balances(mut self, denom: &str, balances: Vec<(&str, u128)>) -> Self {
        self.native_balances
            .extend(balances.into_iter().map(|(addr, amount)| {
                (
                    Addr::unchecked(addr),
                    Coin {
                        denom: denom.to_owned(),
                        amount: amount.into(),
                    },
                )
            }));
        self
    }

    pub fn with_cw20_balances(mut self, balances: Vec<(&str, u128)>) -> Self {
        let initial_balances = balances
            .into_iter()
            .map(|(address, amount)| Cw20Coin {
                address: address.to_owned(),
                amount: amount.into(),
            })
            .collect::<Vec<Cw20Coin>>();
        self.cw20_balances = initial_balances;
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        
        // Default app
        let mut app: App = AppBuilder::new().build(|_, _, _| {});

        // Instantiate2 version
        // prepare wasm module with custom address generator
        // let wasm_keeper: WasmKeeper<Empty, Empty> =
        //     WasmKeeper::new().with_address_generator(MockAddressGenerator);

        // prepare application with custom api
        let mut app = AppBuilder::new()
        .with_wasm::<WasmKeeper<Empty, Empty>>(
            WasmKeeper::new().with_address_generator(MockAddressGenerator),
        )
        .with_api(MockApiBech32::new("migaloo"))
        .build(|_, _, _| {});
        // provide initial native balances
        app.init_modules(|router, _, storage| {
            // group by address
            let mut balances = HashMap::<Addr, Vec<Coin>>::new();
            for (addr, coin) in self.native_balances {
                let addr_balance = balances.entry(addr).or_default();
                addr_balance.push(coin);
            }

            for (addr, coins) in balances {
                router
                    .bank
                    .init_balance(storage, &addr, coins)
                    .expect("init balance");
            }
        });

        let admin = Addr::unchecked("admin");
        let test_account = app.api().addr_make("addr0000");
        let pool_manager_id = contract_pool_manager(&mut app);
        let token_contract_code_id = store_token_code(&mut app);

        let pool_manager_addr = app
            .instantiate_contract(
                pool_manager_id,
                admin.clone(),
                &InstantiateMsg {
                    fee_collector_addr: app.api().addr_make("fee_collector_addr").to_string(),
                    token_code_id: token_contract_code_id,
                    pair_code_id: token_contract_code_id,
                    owner: app.api().addr_make("owner").to_string(),
                    pool_creation_fee: Asset {
                        amount: Uint128::from(100u128),
                        info: AssetInfo::NativeToken {
                            denom: "uusd".to_string(),
                        },
                    },
                },
                &[],
                "pool_manager",
                None,
            )
            .unwrap();

        Suite {
            app,
            pool_manager_addr,
            test_account: test_account,
        }
    }
}

pub struct Suite {
    pub app: App<BankKeeper, MockApiBech32>,
    pub pool_manager_addr: Addr,
    pub test_account: Addr,
}

impl Suite {
    pub fn create_constant_product_pool(
        &mut self,
        sender: Addr,
        asset_infos_array: Vec<AssetInfo>,
        pool_creation_fee: Uint128,
    ) -> AnyResult<AppResponse> {
        // Convert the Vec<AssetInfo> into a [AssetInfo; 2]
        let mut asset_infos_array = [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "fable".to_string(),
            },
        ];
        let msg = ExecuteMsg::CreatePair {
            asset_infos: asset_infos_array.to_vec(),
            pool_fees: PoolFee {
                protocol_fee: Fee {
                    share: Decimal::zero(),
                },
                swap_fee: Fee {
                    share: Decimal::zero(),
                },
                burn_fee: Fee {
                    share: Decimal::zero(),
                },
            },
            pair_type: PairType::ConstantProduct,
            token_factory_lp: false,
            pair_identifier: None,
        };

        let res = self
            .app
            .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &[Coin{
                denom: "uusd".to_string(),
                amount: pool_creation_fee
            }])?;
        Ok(res)
    }

    pub(crate) fn add_liquidity(
        &mut self,
        sender: Addr,
        vec: Vec<Asset>,
        funds: &Vec<Coin>,
        pair_identifier: String,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::ProvideLiquidity {
            assets: vec,
            slippage_tolerance: None,
            receiver: None,
            pair_identifier
        };

        let res = self
            .app
            .execute_contract(sender, self.pool_manager_addr.clone(), &msg, funds)?;
        Ok(res)
    }

    pub(crate) fn withdraw_liquidity(
        &mut self,
        sender: Addr,
        vec: Vec<Asset>,
        funds: &Vec<Coin>,
        pair_identifier: String,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::WithdrawLiquidity{
            assets: vec,
            pair_identifier
        };

        let res = self
            .app
            .execute_contract(sender, self.pool_manager_addr.clone(), &msg, funds)?;
        Ok(res)
    }

    pub(crate) fn add_native_token_decimals(
        &mut self,
        sender: Addr,
        denom: String,
        decimals: u8,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::AddNativeTokenDecimals {
            denom: denom.clone(),
            decimals,
        };
        let res = self
            .app
            .execute_contract(
                sender,
                self.pool_manager_addr.clone(),
                &msg,
                &[Coin {
                    denom: denom.to_string(),
                    amount: Uint128::from(1u128),
                }],
            )
            .unwrap();
        Ok(res)
    }
}

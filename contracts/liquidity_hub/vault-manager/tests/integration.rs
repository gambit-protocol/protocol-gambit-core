extern crate core;

use std::cell::RefCell;

use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, BankMsg, CosmosMsg, Decimal, Uint128, WasmMsg,
};
use cw_ownable::OwnershipError;

use vault_manager::ContractError;
use white_whale_std::fee::Fee;
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairType};
use white_whale_std::pool_network::pair::PoolFee;
use white_whale_std::vault_manager::{
    AssetQueryParams, FilterVaultBy, LpTokenType, PaybackAssetResponse, VaultFee,
};

use crate::common::suite::TestingSuite;
use crate::common::MOCK_CONTRACT_ADDR;

mod common;

#[test]
fn instantiate_vault_manager_successful() {
    let mut suite = TestingSuite::default_with_balances(vec![]);

    suite.create_cw20_token();

    let cw20_token = suite.cw20_tokens[0].clone().into_string();

    suite.instantiate_err(
        LpTokenType::TokenFactory,
        Asset {
            info: AssetInfo::Token {
                contract_addr: cw20_token.clone(),
            },
            amount: Uint128::new(1_000u128),
        },
        |error| {
            assert!(error
                .root_cause()
                .to_string()
                .contains("Vault creation fee must be native token"));
        },
    );

    suite.instantiate(
        MOCK_CONTRACT_ADDR.to_string(),
        LpTokenType::TokenFactory,
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(1_000u128),
        },
    );

    let cw20_code_id = suite.create_cw20_token();
    suite.instantiate(
        MOCK_CONTRACT_ADDR.to_string(),
        LpTokenType::Cw20(cw20_code_id),
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(1_000u128),
        },
    );
}

#[test]
fn verify_ownership() {
    let mut suite = TestingSuite::default_with_balances(vec![]);
    let creator = suite.creator();
    let other = suite.senders[1].clone();
    let unauthorized = suite.senders[2].clone();

    suite
        .instantiate_default()
        .query_ownership(|result| {
            let ownership = result.unwrap();
            assert_eq!(Addr::unchecked(ownership.owner.unwrap()), creator);
        })
        .update_ownership(
            unauthorized,
            cw_ownable::Action::TransferOwnership {
                new_owner: other.to_string(),
                expiry: None,
            },
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::OwnershipError { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::OwnershipError"),
                }
            },
        )
        .update_ownership(
            creator,
            cw_ownable::Action::TransferOwnership {
                new_owner: other.to_string(),
                expiry: None,
            },
            |result| {
                result.unwrap();
            },
        )
        .update_ownership(
            other.clone(),
            cw_ownable::Action::AcceptOwnership,
            |result| {
                result.unwrap();
            },
        )
        .query_ownership(|result| {
            let ownership = result.unwrap();
            assert_eq!(Addr::unchecked(ownership.owner.unwrap()), other);
        })
        .update_ownership(
            other.clone(),
            cw_ownable::Action::RenounceOwnership,
            |result| {
                result.unwrap();
            },
        )
        .query_ownership(|result| {
            let ownership = result.unwrap();
            assert!(ownership.owner.is_none());
        });
}

#[test]
fn create_vaults() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "factory/creator/uLP".to_string()),
        coin(1_000_000_000u128, "factory/another_creator/uLP".to_string()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite
        .instantiate_with_cw20_lp_token()
        .create_vault(
            creator.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            None,
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 2000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
            },
            vec![coin(900u128, "uwhale".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::InvalidVaultCreationFee { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::InvalidVaultCreationFee"
                    ),
                }
            },
        )
        .create_vault(
            creator,
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            None,
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 2000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .create_vault(
            other.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            Some("cheaper_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 20000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 10000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .create_vault(
            other,
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            Some("cheaper_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 20000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 10000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::ExistingVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::ExistingVault"),
                }
            },
        )
        .query_vaults(None, None, |result| {
            let vaults_response = result.unwrap();

            assert_eq!(vaults_response.vaults.len(), 2);
            assert_eq!(vaults_response.vaults[0].identifier, "0".to_string());
            assert_eq!(
                vaults_response.vaults[1].identifier,
                "cheaper_vault".to_string()
            );
        });
}

#[test]
fn deposit_withdraw() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "uluna".to_string()),
        coin(1_000_000_000u128, "factory/creator/uLP".to_string()),
        coin(1_000_000_000u128, "factory/another_creator/uLP".to_string()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    let vault_lp_addr = RefCell::new(AssetInfo::Token {
        contract_addr: "".to_string(),
    });

    suite
        .instantiate_with_cw20_lp_token()
        .create_vault(
            creator.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            None,
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 2000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .update_config(
            creator.clone(),
            None,
            None,
            None,
            None,
            Some(false),
            Some(false),
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uluna".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .update_config(
            creator.clone(),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "unexisting_vault".to_string(),
            vec![coin(5_000u128, "uluna".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NonExistentVault"),
                }
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uluna".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(3_000u128, "uwhale".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FundsMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FundsMismatch"),
                }
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            "0".to_string(),
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::InvalidInitialLiquidityAmount { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::InvalidInitialLiquidityAmount"),
                }
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_vault(FilterVaultBy::Identifier("0".to_string()), |result| {
            let vault_response = result.unwrap();
            let vault = vault_response.vaults.get(0).unwrap();
            *vault_lp_addr.borrow_mut() = vault.lp_asset.clone();

            assert_eq!(
                vault.asset,
                Asset {
                    amount: Uint128::new(5_000u128),
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                }
            );
        })
        .query_vault(FilterVaultBy::Asset(AssetQueryParams {
            asset_info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            start_after: None,
            limit: None,
        }), |result| {
            let vault_response = result.unwrap();
            let vault = vault_response.vaults.get(0).unwrap();
            *vault_lp_addr.borrow_mut() = vault.lp_asset.clone();

            assert_eq!(
                vault.asset,
                Asset {
                    amount: Uint128::new(5_000u128),
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                }
            );
        })
        .query_vault(FilterVaultBy::LpAsset(vault_lp_addr.clone().into_inner()), |result| {
            let vault_response = result.unwrap();
            let vault = vault_response.vaults.get(0).unwrap();
            *vault_lp_addr.borrow_mut() = vault.lp_asset.clone();

            assert_eq!(
                vault.asset,
                Asset {
                    amount: Uint128::new(5_000u128),
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                }
            );
        })
        .query_share(Asset { info: vault_lp_addr.clone().into_inner(), amount: Uint128::new(1_000u128) }, |result| {
            let response = result.unwrap();
            assert_eq!(
                response.share,
                Asset {
                    amount: Uint128::new(1_000u128),
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                }
            );
        });

    let vault_manager = suite.vault_manager_addr.clone();
    let random_cw20_token = suite.cw20_tokens.get(0).unwrap().clone();

    suite
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            creator.clone(),
            |result| {
                // 4k to the user
                assert_eq!(result, Uint128::new(4_000u128));
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            vault_manager.clone(),
            |result| {
                //  1k in vault
                assert_eq!(result, Uint128::new(1_000u128));
            },
        )
        .deposit(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "0".to_string(),
            vec![coin(5_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            other.clone(),
            |result| {
                assert_eq!(result, Uint128::new(5_000u128));
            },
        )
        .withdraw(
            other.clone(),
            Asset {
                info: vault_lp_addr.clone().into_inner(),
                amount: Uint128::new(2_000u128),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .withdraw(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "factory/another_creator/uLP".to_string(),
                },
                amount: Uint128::new(2_000u128),
            },
            vec![coin(2_000u128, "factory/another_creator/uLP".to_string())],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NonExistentVault"),
                }
            },
        )
        .update_config(
            creator.clone(),
            None,
            None,
            None,
            None,
            None,
            Some(true),
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .withdraw(
            other.clone(),
            Asset {
                info: AssetInfo::Token {
                    contract_addr: random_cw20_token.to_string(),
                },
                amount: Uint128::new(2_000u128),
            },
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NonExistentVault"),
                }
            },
        )
        .withdraw(
            other.clone(),
            Asset {
                info: vault_lp_addr.clone().into_inner(),
                amount: Uint128::new(2_000u128),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            other.clone(),
            |result| {
                assert_eq!(result, Uint128::new(3_000u128));
            },
        )
        .withdraw(
            creator.clone(),
            Asset {
                info: vault_lp_addr.clone().into_inner(),
                amount: Uint128::new(4_000u128),
            },
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_balance(
            vault_lp_addr.clone().into_inner(),
            creator.clone(),
            |result| {
                assert_eq!(result, Uint128::zero());
            },
        );

    // create cw20 vault
    suite
        .create_vault(
            creator.clone(),
            AssetInfo::Token {
                contract_addr: random_cw20_token.clone().to_string(),
            },
            Some("cw_20_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 2000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .increase_allowance(
            creator.clone(),
            Addr::unchecked(random_cw20_token.clone()),
            Uint128::new(5_000u128),
            vault_manager.clone(),
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::Token {
                    contract_addr: random_cw20_token.clone().to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            "cw_20_vault".to_string(),
            vec![],
            |result| {
                result.unwrap();
            },
        );
}

#[test]
pub fn update_config() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "uluna".to_string()),
        coin(1_000_000_000u128, "factory/creator/uLP".to_string()),
        coin(1_000_000_000u128, "factory/another_creator/uLP".to_string()),
    ]);

    let creator = suite.creator();
    let unauthorized = suite.senders[2].clone();

    let initial_config = RefCell::new(white_whale_std::vault_manager::Config {
        lp_token_type: LpTokenType::TokenFactory,
        whale_lair_addr: Addr::unchecked(""),
        vault_creation_fee: Asset {
            info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            amount: Default::default(),
        },
        flash_loan_enabled: true,
        deposit_enabled: true,
        withdraw_enabled: true,
    });
    suite
        .instantiate_with_cw20_lp_token()
        .query_config(|response| {
            let config = response.unwrap();
            *initial_config.borrow_mut() = config;
        })
        .update_config(
            unauthorized.clone(),
            None,
            None,
            None,
            None,
            Some(false),
            Some(false),
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                if err != ContractError::OwnershipError(OwnershipError::NotOwner) {
                    panic!("Wrong error type, should return OwnershipError::NotOwner");
                }
            },
        )
        .update_config(
            creator.clone(),
            Some(Addr::unchecked("migaloo1gqjwmexg70ajk439ckfjq0uw2k3u2qmqwy6axu").to_string()),
            Some(Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000u128),
            }),
            Some(123),
            Some(false),
            Some(false),
            Some(false),
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .query_config(|response| {
            let new_config = response.unwrap();
            assert_ne!(new_config, initial_config.borrow().clone());
            assert_eq!(
                new_config,
                white_whale_std::vault_manager::Config {
                    lp_token_type: LpTokenType::Cw20(123),
                    whale_lair_addr: Addr::unchecked(
                        "migaloo1gqjwmexg70ajk439ckfjq0uw2k3u2qmqwy6axu"
                    ),
                    vault_creation_fee: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(5_000u128),
                    },
                    flash_loan_enabled: false,
                    deposit_enabled: false,
                    withdraw_enabled: false,
                }
            );
        });

    suite.instantiate_default().update_config(
        creator.clone(),
        Some(Addr::unchecked("migaloo1gqjwmexg70ajk439ckfjq0uw2k3u2qmqwy6axu").to_string()),
        Some(Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(5_000u128),
        }),
        Some(123),
        Some(false),
        Some(false),
        Some(false),
        vec![],
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::InvalidLpTokenType { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::InvalidLpTokenType"),
            }
        },
    );
}

#[test]
pub fn successful_flashloan() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "uluna".to_string()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite
        .instantiate_with_cw20_lp_token()
        .create_vault(
            creator.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            Some("whale_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(2u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(100_000_000u128),
            },
            "whale_vault".to_string(),
            vec![coin(100_000_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        );

    // create pools to arb
    suite
        .create_pool(
            [
                AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            ],
            [6u8, 6u8],
            PoolFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                swap_fee: Fee {
                    share: Default::default(),
                },
                burn_fee: Fee {
                    share: Default::default(),
                },
            },
            PairType::ConstantProduct,
            false,
        )
        .create_pool(
            [
                AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            ],
            [6u8, 6u8],
            PoolFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                swap_fee: Fee {
                    share: Default::default(),
                },
                burn_fee: Fee {
                    share: Default::default(),
                },
            },
            PairType::ConstantProduct,
            false,
        );

    let balanced_pool = suite.pools[0].clone();
    let skewed_pool = suite.pools[1].clone();

    let amount_balanced_pool = RefCell::new(Uint128::zero());
    let amount_skewed_pool = RefCell::new(Uint128::zero());

    let other_balance = RefCell::new(Uint128::zero());

    suite.query_balance(
        AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
        other.clone(),
        |result| {
            *other_balance.borrow_mut() = result;
        },
    );

    // arb the pool with a flashloan

    // step 1 -> 50_000 whale for 95_238 luna on pool 1
    // step 2 -> 95_238 luna for 86_956 whale on pool 2
    // step 3 -> repay loan, pocket the difference which is 86_956 - 50_000 - 150 (fees) = 36_806 -> profit

    suite
        .provide_liquidity(
            creator.clone(),
            [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::new(1_000_000),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::new(1_000_000),
                },
            ],
            balanced_pool.clone(),
            &vec![
                coin(1_000_000u128, "uluna".to_string()),
                coin(1_000_000u128, "uwhale".to_string()),
            ],
            |result| {
                result.unwrap();
            },
        )
        .provide_liquidity(
            creator.clone(),
            [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::new(2_000_000),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::new(1_000_000),
                },
            ],
            skewed_pool.clone(),
            &vec![
                coin(2_000_000u128, "uluna".to_string()),
                coin(1_000_000u128, "uwhale".to_string()),
            ],
            |result| {
                result.unwrap();
            },
        )
        .simulate_swap(
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            balanced_pool.clone(),
            |result| {
                *amount_balanced_pool.borrow_mut() = result.unwrap().return_amount;
            },
        )
        .simulate_swap(
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            skewed_pool.clone(),
            |result| {
                *amount_skewed_pool.borrow_mut() = result.unwrap().return_amount;
            },
        )
        .query_vault(
            FilterVaultBy::Identifier("whale_vault".to_string()),
            |result| {
                let vault_response = result.unwrap();
                let vault = vault_response.vaults.get(0).unwrap();

                assert_eq!(
                    vault.asset,
                    Asset {
                        amount: Uint128::new(100_000_000u128),
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string()
                        },
                    }
                );
            },
        )
        .query_payback(
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(500_000),
            },
            "whale_vault".to_string(),
            |result| {
                let payback = result.unwrap();
                assert_eq!(
                    payback,
                    PaybackAssetResponse {
                        asset_info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string()
                        },
                        payback_amount: Uint128::new(501_500u128),
                        protocol_fee: Uint128::new(500u128),
                        flash_loan_fee: Uint128::new(1_000u128),
                    }
                );
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000u128),
            },
            "whale_vault".to_string(),
            vec![
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: skewed_pool.to_string(),
                    msg: to_json_binary(&white_whale_std::pool_network::pair::ExecuteMsg::Swap {
                        offer_asset: Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::new(50_000),
                        },
                        max_spread: Some(Decimal::percent(50u64)),
                        belief_price: None,
                        to: None,
                    })
                    .unwrap(),
                    funds: vec![coin(50_000u128, "uwhale".to_string())],
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: balanced_pool.to_string(),
                    msg: to_json_binary(&white_whale_std::pool_network::pair::ExecuteMsg::Swap {
                        offer_asset: Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uluna".to_string(),
                            },
                            amount: amount_skewed_pool.clone().into_inner(),
                        },
                        max_spread: Some(Decimal::percent(50u64)),
                        belief_price: None,
                        to: None,
                    })
                    .unwrap(),
                    funds: vec![coin(
                        amount_skewed_pool.clone().into_inner().u128(),
                        "uluna".to_string(),
                    )],
                }),
            ],
            |result| {
                result.unwrap();
            },
        )
        .query_vault(
            FilterVaultBy::Identifier("whale_vault".to_string()),
            |result| {
                let vault_response = result.unwrap();
                let vault = vault_response.vaults.get(0).unwrap();

                assert_eq!(
                    vault.asset,
                    Asset {
                        amount: Uint128::new(100_000_100u128), // the original amount + flashloan fees
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string()
                        },
                    }
                );
            },
        )
        .query_balance(
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            other.clone(),
            |result| {
                assert_eq!(
                    result,
                    other_balance.clone().into_inner() + Uint128::new(36_806)
                ); // original amount before flashloan + profits
            },
        );
}

#[test]
pub fn unsuccessful_flashloan() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "uluna".to_string()),
    ]);

    let creator = suite.creator();
    let other = suite.senders[1].clone();

    suite.instantiate_with_cw20_lp_token().create_cw20_token();

    let cw20_token = suite.cw20_tokens.get(0).unwrap().clone();
    let vault_manager = suite.vault_manager_addr.clone();
    // create some vaults

    suite
        .create_vault(
            creator.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            Some("whale_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(2u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(100_000_000u128),
            },
            "whale_vault".to_string(),
            vec![coin(100_000_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .create_vault(
            creator.clone(),
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            Some("luna_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(2u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(100_000_000u128),
            },
            "luna_vault".to_string(),
            vec![coin(100_000_000u128, "uluna".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .create_vault(
            creator.clone(),
            AssetInfo::Token {
                contract_addr: cw20_token.clone().to_string(),
            },
            Some("cw20_vault".to_string()),
            VaultFee {
                protocol_fee: Fee {
                    share: Decimal::from_ratio(1u128, 1000u128),
                },
                flash_loan_fee: Fee {
                    share: Decimal::from_ratio(2u128, 1000u128),
                },
            },
            vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .increase_allowance(
            creator.clone(),
            cw20_token.clone(),
            Uint128::new(100_000_000u128),
            vault_manager.clone(),
        )
        .deposit(
            creator.clone(),
            Asset {
                info: AssetInfo::Token {
                    contract_addr: cw20_token.clone().to_string(),
                },
                amount: Uint128::new(100_000_000u128),
            },
            "cw20_vault".to_string(),
            vec![],
            |result| {
                result.unwrap();
            },
        );

    // create pools to arb
    suite
        .create_pool(
            [
                AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            ],
            [6u8, 6u8],
            PoolFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                swap_fee: Fee {
                    share: Default::default(),
                },
                burn_fee: Fee {
                    share: Default::default(),
                },
            },
            PairType::ConstantProduct,
            false,
        )
        .create_pool(
            [
                AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            ],
            [6u8, 6u8],
            PoolFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                swap_fee: Fee {
                    share: Default::default(),
                },
                burn_fee: Fee {
                    share: Default::default(),
                },
            },
            PairType::ConstantProduct,
            false,
        );

    let balanced_pool = suite.pools[0].clone();
    let skewed_pool = suite.pools[1].clone();

    let amount_balanced_pool = RefCell::new(Uint128::zero());
    let amount_skewed_pool = RefCell::new(Uint128::zero());

    let other_balance = RefCell::new(Uint128::zero());

    suite.query_balance(
        AssetInfo::NativeToken {
            denom: "uwhale".to_string(),
        },
        other.clone(),
        |result| {
            *other_balance.borrow_mut() = result;
        },
    );

    // arb the pool with a flashloan

    // step 1 -> 50_000 whale for 95_238 luna on pool 1
    // step 2 -> 95_238 luna for 86_956 whale on pool 2
    // step 3 -> repay loan, pocket the difference which is 86_956 - 50_000 - 1500 (fees) = 35_456 -> profit

    suite
        .provide_liquidity(
            creator.clone(),
            [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::new(1_000_000),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::new(1_000_000),
                },
            ],
            balanced_pool.clone(),
            &vec![
                coin(1_000_000u128, "uluna".to_string()),
                coin(1_000_000u128, "uwhale".to_string()),
            ],
            |result| {
                result.unwrap();
            },
        )
        .provide_liquidity(
            creator.clone(),
            [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                    amount: Uint128::new(2_000_000),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::new(1_000_000),
                },
            ],
            skewed_pool.clone(),
            &vec![
                coin(2_000_000u128, "uluna".to_string()),
                coin(1_000_000u128, "uwhale".to_string()),
            ],
            |result| {
                result.unwrap();
            },
        )
        .simulate_swap(
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            balanced_pool.clone(),
            |result| {
                *amount_balanced_pool.borrow_mut() = result.unwrap().return_amount;
            },
        )
        .simulate_swap(
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            skewed_pool.clone(),
            |result| {
                *amount_skewed_pool.borrow_mut() = result.unwrap().return_amount;
            },
        )
        .query_vault(
            FilterVaultBy::Identifier("whale_vault".to_string()),
            |result| {
                let vault_response = result.unwrap();
                let vault = vault_response.vaults.get(0).unwrap();

                assert_eq!(
                    vault.asset,
                    Asset {
                        amount: Uint128::new(100_000_000u128),
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string()
                        },
                    }
                );
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(500_000),
            },
            "unexisting_vault".to_string(),
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentVault { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NonExistentVault"),
                }
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(500_000),
            },
            "whale_vault".to_string(),
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::AssetMismatch { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::AssetMismatch"),
                }
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000_000_000_000),
            },
            "whale_vault".to_string(),
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::InsufficientAssetBalance { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::InsufficientAssetBalance"
                    ),
                }
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            "whale_vault".to_string(),
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NegativeProfit { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::NegativeProfit"),
                }
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            "whale_vault".to_string(),
            vec![
                // try to drain a native token vault
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: other.clone().to_string(),
                    amount: coins(100_000_000u128, "uluna"),
                }),
            ],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                // This should be failing when doing
                // `if original_native_assets_count > new_balances.len()` in the after_flashloan() function.
                // However it does not because the BankKeeper mock returns 0 for denoms that are not
                // in the balances map, which in the cosmos sdk doesn't happen as it only returns
                // the non-zero values.
                match err {
                    ContractError::FlashLoanLoss { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlashLoanLoss"),
                }
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            "whale_vault".to_string(),
            vec![
                // try to drain the cw20 token vault
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20_token.clone().to_string(),
                    msg: to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: other.clone().to_string(),
                        amount: Uint128::new(100_000_000u128),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
            ],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlashLoanLoss { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlashLoanLoss"),
                }
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            "whale_vault".to_string(),
            vec![
                // try to drain the cw20 token vault
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: vault_manager.clone().to_string(),
                    msg: to_json_binary(&white_whale_std::vault_manager::ExecuteMsg::Deposit {
                        asset: Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::new(50_000),
                        },
                        vault_identifier: "whale_vault".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                }),
            ],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .update_config(
            creator.clone(),
            None,
            None,
            None,
            Some(false),
            None,
            None,
            vec![],
            |result| {
                result.unwrap();
            },
        )
        .flashloan(
            other.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(50_000),
            },
            "whale_vault".to_string(),
            vec![],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::Unauthorized { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
                }
            },
        )
        .callback(other.clone(), |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::Unauthorized { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
            }
        });

    suite
        .query_payback(
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000_000_000_000),
            },
            "whale_vault".to_string(),
            |result| {
                assert!(result.unwrap_err().to_string().contains(
                    "The requested vault doesn't have enough balance to serve the demand"
                ));
            },
        )
        .query_payback(
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(5_000_000_000_000),
            },
            "non_existent_vault".to_string(),
            |result| {
                assert!(result
                    .unwrap_err()
                    .to_string()
                    .contains("Vault doesn't exist"));
            },
        );
}
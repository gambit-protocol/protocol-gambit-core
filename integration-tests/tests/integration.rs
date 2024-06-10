use bonding_manager::ContractError;
use common::suite::{ampWHALE, bWHALE};
use cosmwasm_std::{coin, coins};

use white_whale_std::pool_manager::PoolType;

use crate::common::helpers;
use crate::common::suite::TestingSuite;

mod common;

#[test]
fn epic_test() {
    let mut suite = TestingSuite::default_with_balances();
    suite.instantiate();

    let [alice, bob, carol, dave, sybil] = [
        suite.senders[0].clone(),
        suite.senders[1].clone(),
        suite.senders[2].clone(),
        suite.senders[3].clone(),
        suite.senders[4].clone(),
    ];

    // create some pools, vaults, incentives
    helpers::pools::create_pools(&mut suite, alice.clone());
    helpers::vaults::create_vaults(&mut suite, bob.clone());
    helpers::incentives::create_incentives(&mut suite, carol.clone());

    suite
        // cannot bond if rewards bucket is empty
        .bond(alice.clone(), &coins(10_000, ampWHALE), |result| {
            assert_eq!(
                result
                    .unwrap_err()
                    .downcast::<bonding_manager::ContractError>()
                    .unwrap(),
                bonding_manager::ContractError::RewardBucketIsEmpty
            );
        })
        // cannot swap before adding liquidity
        .swap(
            carol.clone(),
            "uusdc".to_string(),
            None,
            None,
            None,
            "uwhale-uusdc-cheap".to_string(),
            coins(10_000, "uwhale"),
            |result| {
                assert_eq!(
                    result
                        .unwrap_err()
                        .downcast::<pool_manager::ContractError>()
                        .unwrap(),
                    pool_manager::ContractError::PoolHasNoAssets
                );
            },
        )
        .provide_liquidity(
            alice.clone(),
            "uwhale-uusdc-cheap".to_string(),
            None,
            None,
            None,
            None,
            vec![coin(100_000_000, "uwhale"), coin(100_000_000, "uusdc")],
            |result| {
                result.unwrap();
            },
        )
        .swap(
            carol.clone(),
            "uusdc".to_string(),
            None,
            None,
            None,
            "uwhale-uusdc-cheap".to_string(),
            coins(10_000, "uwhale"),
            |result| {
                result.unwrap();
            },
        )
        .add_one_epoch()
        .bond(alice, &coins(10_000, ampWHALE), |result| {
            result.unwrap();
        })
        .bond(bob, &coins(40_000, bWHALE), |result| {
            result.unwrap();
        })
        .swap(
            carol,
            "uusdc".to_string(),
            None,
            None,
            None,
            "uwhale-uusdc-cheap".to_string(),
            coins(10_000, "uwhale"),
            |result| {
                result.unwrap();
            },
        );
}

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
        // before we start doing anything, let's make sure we are in epoch 1
        .query_current_epoch(|response| {
            assert_eq!(response.unwrap().epoch.id, 1);
        })
        // claimable rewards should be empty
        .query_claimable_reward_buckets(None, |response| {
            assert!(response.unwrap().1.is_empty());
        })
        // create 1 epoch
        .add_one_epoch()
        // claimable rewards should have 19_000 uwhale due to the initial setup (on epoch 1)
        .query_claimable_reward_buckets(None, |response| {
            assert_eq!(response.unwrap().1[0].available[0], coin(19_000, "uwhale"));
        })
        // bond alice with 10_000 uwhale on epoch 2 (without swapping)
        .bond(&alice, &coins(10_000, ampWHALE), |result| {
            result.unwrap();
        })
        // create 20 more epochs, should not let alice claim any rewards
        .add_epochs(20)
        .query_current_epoch(|result| {
            assert_eq!(result.unwrap().epoch.id, 22);
        })
        .query_claimable_reward_buckets(Some(&alice), |response| {
            assert!(response.unwrap().1.is_empty());
        })
        // create 1 more epoch should let alice claim 19_000 uwhale from the initial setup
        .add_epochs(1)
        .query_current_epoch(|result| {
            assert_eq!(result.unwrap().epoch.id, 23);
        })
        .query_claimable_reward_buckets(Some(&alice), |response| {
            assert_eq!(response.unwrap().1[0].available, coins(19_000, "uwhale"));
        })
        .query_bonding_rewards(alice.to_string(), |response| {
            assert_eq!(response.unwrap().1.rewards, coins(19_000, "uwhale"));
        })
        // claim the rewards
        .claim_bonding_rewards(&alice, |result| {
            result.unwrap();
        })
        // should not be able to claim the same rewards again
        .claim_bonding_rewards(&alice, |result| {
            assert_eq!(
                result.unwrap_err().downcast::<ContractError>().unwrap(),
                ContractError::NothingToClaim
            );
        })
        // check that the rewards are claimed
        .query_claimable_reward_buckets(Some(&alice), |response| {
            assert!(response.unwrap().1.is_empty());
        })
        .query_claimable_reward_buckets(None, |response| {
            assert!(response.unwrap().1.is_empty());
        })
        .query_bonding_rewards(alice.to_string(), |response| {
            assert!(response.unwrap().1.rewards.is_empty());
        })
        // move to epoch 24
        .add_one_epoch()
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
        // check we're on epoch 24
        .query_current_epoch(|result| {
            assert_eq!(result.unwrap().epoch.id, 24);
        })
        .swap(
            carol.clone(),
            "uusdc".to_string(),
            None,
            None,
            None,
            "uwhale-uusdc-cheap".to_string(),
            coins(100_000, "uwhale"),
            |result| {
                result.unwrap();
            },
        )
        // alice should still have 0 uwhale claimable rewards
        .query_claimable_reward_buckets(Some(&alice), |response| {
            assert!(response.unwrap().1.is_empty());
        })
        .add_one_epoch()
        // bond bob with 40_000 uwhale on epoch 25
        .bond(&bob, &coins(40_000, ampWHALE), |result| {
            result.unwrap();
        })
        // bob should have 0 uwhale claimable rewards
        .query_claimable_reward_buckets(Some(&bob), |response| {
            assert!(response.unwrap().1.is_empty());
        })
        // alice should have X claimable rewards
        .query_claimable_reward_buckets(Some(&alice), |response| {
            // assert!(response.as_ref().unwrap().1[0].available.is_empty());
            println!("{:?}", response.unwrap().1);
            // assert_eq!(response.unwrap().1[0].available[0], coin(19_000, "uwhale"));
        })
        .add_one_epoch()
        .query_claimable_reward_buckets(Some(&alice), |response| {
            println!("{:?}", response.unwrap().1);
        })
        .query_claimable_reward_buckets(Some(&bob), |response| {
            println!("{:?}", response.unwrap().1)
        })
        .query_bonding_rewards(alice.to_string(), |response| {
            println!("{:?}", response.unwrap().1);
        })
        .swap(
            carol.clone(),
            "uwhale".to_string(),
            None,
            None,
            None,
            "uwhale-uusdc-cheap".to_string(),
            coins(100_000, "uusdc"),
            |result| {
                result.unwrap();
            },
        )
        .query_bonding_rewards(alice.to_string(), |response| {
            println!("{:?}", response.unwrap().1);
        })
        .add_one_epoch()
        .query_claimable_reward_buckets(Some(&bob), |response| {
            println!("{:?}", response.unwrap().1);
        });
}

#[test]
fn epic_test_tshoot() {
    let mut suite = TestingSuite::default_with_balances();
    suite.instantiate();

    let [alice, bob, carol, dave, sybil] = [
        suite.senders[0].clone(),
        suite.senders[1].clone(),
        suite.senders[2].clone(),
        suite.senders[3].clone(),
        suite.senders[4].clone(),
    ];

    let creator = suite.creator().clone();

    // create some pools, vaults, incentives
    helpers::pools::create_pools(&mut suite, alice.clone());
    helpers::vaults::create_vaults(&mut suite, bob.clone());
    helpers::incentives::create_incentives(&mut suite, carol.clone());

    suite
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .add_one_epoch()
        .query_current_epoch(|result| {
            assert_eq!(result.unwrap().epoch.id, 6);
        });
}

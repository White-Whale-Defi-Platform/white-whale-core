use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use bonding_manager::ContractError;
use common::suite::{AMPWHALE, BWHALE};
use cosmwasm_std::{coin, coins};
use cosmwasm_std::{Addr, Coin};

use proptest::prelude::*;
use proptest::prop_oneof;
use proptest::strategy::{Just, Strategy};

use crate::common::helpers;
use crate::common::suite::TestingSuite;

mod common;

#[test]
fn epic_test() {
    let mut suite = TestingSuite::default_with_balances();
    suite.instantiate();

    let [alice, bob, carol, _dave, _sybil] = [
        suite.senders[0].clone(),
        suite.senders[1].clone(),
        suite.senders[2].clone(),
        suite.senders[3].clone(),
        suite.senders[4].clone(),
    ];

    // create some pools, vaults, incentives
    helpers::pools::create_pools(&mut suite, alice.clone());
    helpers::vaults::create_vaults(&mut suite, bob.clone());
    helpers::vaults::add_vault_liquidity(&mut suite, bob.clone());
    helpers::incentives::create_incentives(&mut suite, carol.clone());

    let current_rewards = Rc::new(RefCell::new(0));

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
        .bond(&alice, &coins(10_000, AMPWHALE), |result| {
            result.unwrap();
        })
        // create 20 more epochs, should not let alice claim any rewards
        .add_epochs(100)
        .query_current_epoch(|result| {
            assert_eq!(result.unwrap().epoch.id, 102);
        })
        .query_claimable_reward_buckets(Some(&alice), |response| {
            assert!(response.unwrap().1.is_empty());
        })
        // create 1 more epoch should let alice claim 19_000 uwhale from the initial setup
        .add_epochs(1)
        .query_current_epoch(|result| {
            assert_eq!(result.unwrap().epoch.id, 103);
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
        // check we're on epoch 24
        .query_current_epoch(|result| {
            assert_eq!(result.unwrap().epoch.id, 104);
        })
        .swap(
            carol.clone(),
            "uusdc".to_string(),
            None,
            None,
            None,
            "uwhale-uusdc".to_string(),
            coins(10_000, "uwhale"),
            |result| {
                result.as_ref().unwrap().events.iter().for_each(|event| {
                    // get the protocol fee amount
                    event.attributes.iter().for_each(|attr| {
                        if attr.key == "protocol_fee_amount" {
                            *current_rewards.borrow_mut() += attr.value.parse::<u128>().unwrap();
                            assert_eq!(*current_rewards.borrow(), (10_000. * 0.001 - 1.) as u128);
                        }
                    });
                });
                result.unwrap();
            },
        )
        // alice should still have 0 uwhale claimable rewards
        .query_claimable_reward_buckets(Some(&alice), |response| {
            assert!(response.unwrap().1.is_empty());
        })
        .add_one_epoch()
        // bond bob with 40_000 uwhale on epoch 25
        .bond(&bob, &coins(40_000, AMPWHALE), |result| {
            result.unwrap();
        })
        // bob should have 0 uwhale claimable rewards until a swap is made
        .query_claimable_reward_buckets(Some(&bob), |response| {
            assert!(response.unwrap().1.is_empty());
        })
        // alice should have 10 claimable rewards, 0.1% of the 10_000 uwhale swapped
        .query_claimable_reward_buckets(Some(&alice), |response| {
            assert_eq!(
                response.as_ref().unwrap().1[0].available[0].amount.u128(),
                // TODO: >>> make sure that the 1 uwhale we lose here is due to rounding...
                (10_000. * 0.001 - 1.) as u128
            );
        })
        .add_one_epoch()
        .query_bonding_rewards(alice.to_string(), |response| {
            println!("{:?}", response.unwrap().1);
        })
        .swap(
            carol.clone(),
            "uwhale".to_string(),
            None,
            None,
            None,
            "uwhale-uusdc".to_string(),
            coins(10_000, "uusdc"),
            |result| {
                result.unwrap();
            },
        )
        .add_one_epoch()
        .query_claimable_reward_buckets(Some(&bob), |response| {
            println!("{:?}", response.unwrap().1);
        });

    // check we're on epoch 200
    suite.add_epochs(93);
    suite.query_current_epoch(|result| {
        assert_eq!(result.unwrap().epoch.id, 200);
    });
}

proptest! {
    #[test]
    fn property_based_test(
        actions in proptest::collection::vec(action_strategy(
            vec![
                Addr::unchecked("migaloo1ludaslnu24p5eftw499f7ngsc2jkzqdsrvxt75"),
                Addr::unchecked("migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40"),
                Addr::unchecked("migaloo1lh7mmdavky83xks76ch57whjaqa7e456vvpz8y"),
                Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3"),
                Addr::unchecked("migaloo13y3petsaw4vfchac4frjmuuevjjjcceja7sjx7")
            ]
        ), 1..100)
    ) {
        println!(">>> ------------------- STARTING TEST --------------------");
        let mut suite = TestingSuite::default_with_balances();
        suite.instantiate();

        let [alice, bob, carol, _dave, _sybil] = [
            suite.senders[0].clone(),
            suite.senders[1].clone(),
            suite.senders[2].clone(),
            suite.senders[3].clone(),
            suite.senders[4].clone(),
        ];

        // create some pools, vaults, incentives
        helpers::pools::create_pools(&mut suite, alice.clone());
        helpers::vaults::create_vaults(&mut suite, bob.clone());
        helpers::vaults::add_vault_liquidity(&mut suite, bob.clone());
        helpers::incentives::create_incentives(&mut suite, carol.clone());

        // let's bond with alice and create some epochs to empty the rewards bucket due to the initial setup
        suite.bond(&alice, &coins(10_000, AMPWHALE), |result| {
            result.unwrap();
        });
        suite.add_epochs(20);
        suite.query_bonding_rewards(alice.to_string(), |response| {
            assert_eq!(response.unwrap().1.rewards, coins(19_000, "uwhale"));
        });
        suite.claim_bonding_rewards(&alice, |result| {
            result.unwrap();
        });
        suite.unbond(alice.clone(), coin(10_000, AMPWHALE), |result| {
            result.unwrap();
        });
        suite.add_epochs(1);
        suite.withdraw_after_unbond(alice.clone(), AMPWHALE.to_string(), |result| {
            result.unwrap();
        });
        suite.add_epochs(78);
        suite.query_claimable_reward_buckets(None, |response| {
            assert!(response.unwrap().1.is_empty());
        });

        let current_rewards = Rc::new(RefCell::new(0));
        let bonded_amounts = Rc::new(RefCell::new(HashMap::<Addr, HashMap<String, u128>>::new()));
        // simple counter that increments every time a user unbonds
        let unbond_id_counter = Rc::new(RefCell::new(0));
        // user => (bonded_token, unbond_id_counter) => epoch
        let unbonding_amounts = Rc::new(RefCell::new(HashMap::<Addr, HashMap<(String, u64), u64>>::new()));
        // (user, epoch) => bool
        let claimable_rewards = Rc::new(RefCell::new(HashMap::<(Addr, u64), bool>::new()));
        let available_pools = suite.pool_identifiers.clone();
        // list of the users that claimed in the current epoch (either by executing claim or bond/unbond with pending rewards)
        let claimed = Rc::new(RefCell::new(HashSet::new()));

        let mut swaps_in_epoch = false;

        for action in actions {
            let mut current_epoch = 0;
            suite.query_current_epoch(|response| {
                current_epoch = response.unwrap().epoch.id;
            });

            match action {
                Action::Swap(user, from_token, to_token, amount) => {
                    if from_token == to_token {
                        println!(">>> [{current_epoch}] [{user}] SWAP FAILED [{amount} {from_token} -> {to_token}]");

                        suite.swap(
                            user.clone(),
                            to_token.clone(),
                            None,
                            None,
                            None,
                            "uwhale-uusdc".to_string(),
                            coins(amount, from_token),
                            move |result| {
                                assert_eq!(
                                    result.unwrap_err().downcast::<pool_manager::ContractError>().unwrap(),
                                    pool_manager::ContractError::SameAsset
                                );
                            }
                        );
                    } else {
                        let pool_identifier = create_pool_identifier(&from_token, &to_token);

                        // enter the normal swap flow if the pool exists
                        if available_pools.contains(&pool_identifier) {
                            println!(">>> [{current_epoch}] [{user}] SWAP [{amount} {from_token} -> {to_token}]");

                            swaps_in_epoch = true;
                            suite.swap(
                                user.clone(),
                                to_token.clone(),
                                None,
                                None,
                                None,
                                pool_identifier,
                                coins(amount, from_token),
                                {
                                    let current_rewards = Rc::clone(&current_rewards);
                                    move |result| {
                                        result.as_ref().unwrap().events.iter().for_each(|event| {
                                            // get the protocol fee amount
                                            event.attributes.iter().for_each(|attr| {
                                                if attr.key == "protocol_fee_amount" {
                                                    *current_rewards.borrow_mut() += attr.value.parse::<u128>().unwrap();
                                                }
                                            });
                                        });
                                        result.unwrap();
                                    }
                                },
                            );

                            // Mark claimable rewards for users bonded during this epoch
                            let bonded_amounts = bonded_amounts.borrow();
                            for (user, token_amounts) in bonded_amounts.iter() {
                                if token_amounts.values().any(|&amount| amount > 0) {
                                    claimable_rewards.borrow_mut().insert((user.clone(), current_epoch), true);
                                    //
                                }
                            }
                        } else {
                            suite.swap(
                                user.clone(),
                                to_token.clone(),
                                None,
                                None,
                                None,
                                pool_identifier,
                                coins(amount, from_token),
                                move |result| {
                                    assert_eq!(
                                        result.unwrap_err().downcast::<pool_manager::ContractError>().unwrap(),
                                        pool_manager::ContractError::UnExistingPool
                                    );
                                }
                            );
                        }
                    }
                }
                Action::Bond(user, token, amount) => {
                    // bond the user
                    suite.bond(&user, &coins(amount, &token), |result| {
                        println!(">>> [{current_epoch}] [{user}] BOND [{amount} {}]", token.split('/').nth(2).unwrap());
                        result.unwrap();
                    });

                    let mut test_has_rewards = false;
                    // check if the user has pending rewards in the test map
                    for ((test_user, epoch), has_rewards) in claimable_rewards.borrow().iter() {
                        if test_user == &user && *epoch < current_epoch && *has_rewards {
                            test_has_rewards = true;
                            break;
                        }
                    }

                    // if user bonded with pending rewards, the contract will claim them first so we need to remove them from the test map
                    if test_has_rewards {
                        for epoch in 0..current_epoch {
                            claimable_rewards.borrow_mut().remove(&(user.clone(), epoch));
                        }
                        claimed.borrow_mut().insert(user.clone());
                    }

                    suite.query_bonding_rewards(user.to_string(), |response| {
                        let contract_rewards = response.unwrap().1.rewards;
                        let has_contract_rewards = !contract_rewards.is_empty();

                        // re-calculate if the user has pending rewards in the test map
                        let mut has_rewards_in_test_map = false;
                        for ((test_user, epoch), has_rewards) in claimable_rewards.borrow().iter() {
                            if test_user == &user && *epoch < current_epoch && *has_rewards {
                                has_rewards_in_test_map = true;
                                break;
                            }
                        }

                        assert_eq!(has_rewards_in_test_map, has_contract_rewards);
                    });

                    let mut bonded_amounts = bonded_amounts.borrow_mut();
                    let user_bonds = bonded_amounts.entry(user.clone()).or_insert_with(HashMap::new);
                    *user_bonds.entry(token.clone()).or_insert(0) += amount;

                    if swaps_in_epoch {
                        claimable_rewards.borrow_mut().insert((user.clone(), current_epoch), true);
                    }

                    suite.query_bonded(Some(user.to_string()), |result| {
                        let bonded = result.unwrap().1.bonded_assets;
                        let mut expected_bonded = user_bonds.iter()
                            .map(|(token, amount)| coin(*amount, token))
                            .collect::<Vec<Coin>>();
                        expected_bonded.sort_by(|a, b| a.denom.cmp(&b.denom));
                        let mut bonded_sorted = bonded.clone();
                        bonded_sorted.sort_by(|a, b| a.denom.cmp(&b.denom));
                        assert_eq!(bonded_sorted, expected_bonded);
                    });
                }
                Action::Unbond(user, token, amount) => {
                    let mut bonded_amounts = bonded_amounts.borrow_mut();
                    let user_bonds = bonded_amounts.entry(user.clone()).or_insert_with(HashMap::new);

                    if let Some(bonded) = user_bonds.get_mut(&token) {
                        if *bonded >= amount {
                            println!(">>> [{current_epoch}] [{user}] UNBOND [{amount} {}]", token.split('/').nth(2).unwrap());

                            let mut test_has_rewards = false;
                            // check if the user has pending rewards in the test map
                            for ((test_user, epoch), has_rewards) in claimable_rewards.borrow().iter() {
                                if test_user == &user && *epoch < current_epoch && *has_rewards {
                                    test_has_rewards = true;
                                    break;
                                }
                            }

                            // if user bonded with pending rewards, the contract will claim them first so we need to remove them from the test map
                            if test_has_rewards {
                                for epoch in 0..current_epoch {
                                    claimable_rewards.borrow_mut().remove(&(user.clone(), epoch));
                                }
                                claimed.borrow_mut().insert(user.clone());
                            }

                            suite.unbond(user.clone(), coin(amount, &token), |result| {
                                result.unwrap();
                            });
                            *bonded -= amount;

                            let mut unbonding_amounts = unbonding_amounts.borrow_mut();
                            let user_unbonds = unbonding_amounts.entry(user.clone()).or_insert_with(HashMap::new);
                            let unbond_id = *unbond_id_counter.borrow_mut() + 1;
                            *unbond_id_counter.borrow_mut() = unbond_id;
                            user_unbonds.insert((token.clone(), unbond_id), current_epoch);

                            suite.query_bonding_rewards(user.to_string(), |response| {
                                let contract_rewards = response.unwrap().1.rewards;
                                let has_contract_rewards = !contract_rewards.is_empty();

                                let mut has_rewards_in_test_map = false;
                                for ((test_user, epoch), has_rewards) in claimable_rewards.borrow().iter() {
                                    if test_user == &user && *epoch < current_epoch && *has_rewards {
                                        has_rewards_in_test_map = true;
                                        break;
                                    }
                                }

                                assert_eq!(has_rewards_in_test_map, has_contract_rewards);
                            });
                        } else {
                            println!(">>> [{current_epoch}] [{user}] UNBOND FAILED (insufficient bond) [{amount} {}]", token.split('/').nth(2).unwrap());

                            suite.unbond(user.clone(), coin(amount, &token), |result| {
                                assert_eq!(
                                    result.unwrap_err().downcast::<ContractError>().unwrap(),
                                    ContractError::InsufficientBond
                                );
                            });
                        }
                    } else {
                        println!(">>> [{current_epoch}] [{user}] UNBOND FAILED (nothing to unbond) [{amount} {}]", token.split('/').nth(2).unwrap());

                        suite.unbond(user.clone(), coin(amount, &token), |result| {
                            assert_eq!(
                                result.unwrap_err().downcast::<ContractError>().unwrap(),
                                ContractError::NothingToUnbond
                            );
                        });
                    }
                }
                Action::Withdraw(user, token) => {
                    let mut unbonding_amounts = unbonding_amounts.borrow_mut();
                    let user_unbonds: Vec<((String, u64), u64)> = unbonding_amounts.get(&user)
                        .map(|user_unbonds| {
                            user_unbonds.iter()
                                .filter_map(|(&(ref tok, unbond_id), &epoch)| {
                                    if tok == &token && epoch < current_epoch {
                                        Some(((tok.clone(), unbond_id), epoch))
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    if !user_unbonds.is_empty() {
                        println!(">>> [{current_epoch}] [{user}] WITHDRAW [{} {}]", user_unbonds.len(), token.split('/').nth(2).unwrap());

                        suite.withdraw_after_unbond(user.clone(), token.clone(), |result| {
                            result.unwrap();
                        });

                        // clear withdrawn unbonds from the unbonding_amounts map
                        for ((tok, unbond_id), _) in user_unbonds {
                            unbonding_amounts.get_mut(&user).unwrap().remove(&(tok, unbond_id));
                        }
                    } else {
                        println!(">>> [{current_epoch}] [{user}] WITHDRAW FAILED (nothing to withdraw)");

                        suite.withdraw_after_unbond(user.clone(), token.clone(), |result| {
                            assert_eq!(
                                result.unwrap_err().downcast::<ContractError>().unwrap(),
                                ContractError::NothingToWithdraw
                            );
                        });
                    }
                }
                Action::Claim(user) => {
                    let mut has_pending_rewards = false;
                    for ((test_user, epoch), has_rewards) in claimable_rewards.borrow().iter() {
                        if test_user == &user && *epoch < current_epoch && *has_rewards {
                            has_pending_rewards = true;
                            break;
                        }
                    }

                    if has_pending_rewards {
                        println!(">>> [{current_epoch}] [{user}] CLAIM");

                        suite.claim_bonding_rewards(&user, |result| {
                            result.unwrap();
                        });

                        for epoch in 0..current_epoch {
                            claimable_rewards.borrow_mut().remove(&(user.clone(), epoch));
                        }

                        claimed.borrow_mut().insert(user.clone());
                    } else {
                        println!(">>> [{current_epoch}] [{user}] CLAIM FAILED (nothing to claim)");

                        suite.claim_bonding_rewards(&user, |result| {
                            assert_eq!(
                                result.unwrap_err().downcast::<ContractError>().unwrap(),
                                ContractError::NothingToClaim
                            );
                        });
                    }
                }
            }

            if rand::random() {
                // workaround to avoid hitting an error when unclaimed rewards are rolled over after 21 epochs
                // if there are unclaimed rewards when creating a new epoch, we need to add them to the claimable rewards map +21 epochs ahead
                suite.add_one_epoch();
                swaps_in_epoch = false;
            }
        }
    }
}

fn create_pool_identifier(from_token: &str, to_token: &str) -> String {
    let (token_a, token_b) = if from_token < to_token {
        (from_token, to_token)
    } else {
        (to_token, from_token)
    };

    format!(
        "{}-{}",
        if token_a.starts_with("peggy") {
            "peggy"
        } else {
            token_a
        },
        if token_b.starts_with("peggy") {
            "peggy"
        } else {
            token_b
        }
    )
}

#[derive(Clone, Debug)]
enum Action {
    Swap(Addr, String, String, u128),
    Bond(Addr, String, u128),
    Unbond(Addr, String, u128),
    Withdraw(Addr, String),
    Claim(Addr),
}

fn action_strategy(users: Vec<Addr>) -> impl Strategy<Value = Action> {
    let user_strategy = prop_oneof![
        Just(users[0].clone()),
        Just(users[1].clone()),
        Just(users[2].clone()),
        Just(users[3].clone()),
        Just(users[4].clone())
    ];

    let swap_token_strategy = prop_oneof![
        Just("peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string()),
        Just("ibc/BEFB9AB13AB43157A0AF6254AD4B1F565AC0CA0C1760B8339BE7B9E2996F7752".to_string()),
        Just("btc".to_string()),
        Just("inj".to_string()),
        Just("uwhale".to_string()),
        Just("uusdc".to_string()),
        Just("uusdt".to_string())
    ];

    let bond_unbond_token_strategy =
        prop_oneof![Just(BWHALE.to_string()), Just(AMPWHALE.to_string())];

    const MIN_AMOUNT: u128 = 10_000;
    let amount_strategy = MIN_AMOUNT..100_000_000_u128;

    prop_oneof![
        (
            user_strategy.clone(),
            swap_token_strategy.clone(),
            swap_token_strategy.clone(),
            amount_strategy.clone()
        )
            .prop_map(|(user, from_token, to_token, amount)| Action::Swap(
                user, from_token, to_token, amount
            )),
        (
            user_strategy.clone(),
            bond_unbond_token_strategy.clone(),
            amount_strategy.clone()
        )
            .prop_map(|(user, token, amount)| Action::Bond(user, token, amount)),
        (
            user_strategy.clone(),
            bond_unbond_token_strategy.clone(),
            amount_strategy.clone()
        )
            .prop_map(|(user, token, amount)| Action::Unbond(user, token, amount)),
        (user_strategy.clone(), bond_unbond_token_strategy.clone())
            .prop_map(|(user, token)| Action::Withdraw(user, token)),
        user_strategy.clone().prop_map(|user| Action::Claim(user)),
    ]
}

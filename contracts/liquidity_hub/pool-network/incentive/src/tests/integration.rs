use std::cell::RefCell;

use anyhow::Error;
use cosmwasm_std::{coin, coins, Addr, Timestamp, Uint128};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::incentive;
use white_whale::pool_network::incentive::{Curve, Flow};

use crate::error::ContractError;
use crate::tests::suite::TestingSuite;

#[test]
fn instantiate_incentive_factory_successful() {
    let mut suite = TestingSuite::default();

    suite.instantiate(
        "fee_collector_addr".to_string(),
        Asset {
            amount: Uint128::new(1_000u128),
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        },
        1_000,
        1,
        1_000,
        1_000,
        2_000,
    );
}

#[test]
fn instantiate_incentive_factory_unsuccessful() {
    let mut suite = TestingSuite::default();

    suite.instantiate_err(
        "fee_collector_addr".to_string(),
        Asset {
            amount: Uint128::new(1_000u128),
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        },
        1_000,
        1,
        1_000,
        1_000,
        500,
        |error| {
            let err = error
                .downcast::<incentive_factory::error::ContractError>()
                .unwrap();

            match err {
                incentive_factory::error::ContractError::InvalidUnbondingRange { min, max } => {
                    assert_eq!(min, 1000);
                    assert_eq!(max, 500);
                }
                _ => panic!("Wrong error type, should return ContractError::InvalidUnbondingRange"),
            }
        },
    );
}

#[test]
fn create_incentive() {
    let mut suite =
        TestingSuite::default_with_balances(coins(1_000_000_000u128, "uwhale".to_string()));
    let creator = suite.creator();
    let unauthorized = suite.senders[2].clone();

    suite.instantiate_default().create_lp_tokens();

    let lp_address_1 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };
    let lp_address_2 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.last().unwrap().to_string(),
    };

    // first try to execute anything on the incentive factory contract from a non-owner, it should error
    // then do it with the owner of the contract
    suite
        .create_incentive(unauthorized, lp_address_1.clone(), |result| {
            let err = result
                .unwrap_err()
                .downcast::<incentive_factory::error::ContractError>()
                .unwrap();

            match err {
                incentive_factory::error::ContractError::Unauthorized {} => {}
                _ => panic!("Wrong error type, should return ContractError::Unauthorized"),
            }
        })
        .create_incentive(creator.clone(), lp_address_1.clone(), |result| {
            result.unwrap();
        })
        .query_incentive(lp_address_1.clone(), |result| {
            let incentive = result.unwrap();
            assert!(incentive.is_some());
        })
        // this should error cuz the incentive for that lp was already created
        .create_incentive(creator.clone(), lp_address_1.clone(), |result| {
            let err = result
                .unwrap_err()
                .downcast::<incentive_factory::error::ContractError>()
                .unwrap();

            match err {
                incentive_factory::error::ContractError::DuplicateIncentiveContract { .. } => {}
                _ => panic!(
                    "Wrong error type, should return ContractError::DuplicateIncentiveContract"
                ),
            }
        })
        .create_incentive(creator.clone(), lp_address_2, |result| {
            result.unwrap();
        })
        .query_incentives(None, None, |result| {
            let incentives = result.unwrap();
            assert_eq!(incentives.len(), 2usize);
        });
}

#[test]
fn open_close_flows_from_incentive() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "usdc".to_string()),
    ]);
    let alice = suite.creator();
    let bob = suite.senders[1].clone();
    let carol = suite.senders[2].clone();

    suite.instantiate_default().create_lp_tokens();

    let incentive_factory_addr = suite.incentive_factory_addr.clone();
    let mut fee_collector_addr = RefCell::new(Addr::unchecked(""));

    let lp_address_1 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };

    let mut incentive_addr = RefCell::new(Addr::unchecked(""));

    suite
        .create_incentive(alice.clone(), lp_address_1.clone(), |result| {
            result.unwrap();
        })
        .query_incentive(lp_address_1.clone(), |result| {
            let incentive = result.unwrap();
            assert!(incentive.is_some());
            *incentive_addr.borrow_mut() = incentive.unwrap();
        });

    // open 7 incentives, it should fail on the 8th
    let app_time = suite.get_time();
    for i in 1..=8 {
        suite
            .open_incentive_flow(
                carol.clone(),
                incentive_addr.clone().into_inner(),
                None,
                app_time.clone().plus_seconds(86400u64).seconds(),
                Curve::Linear,
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::new(i * 1_000u128),
                },
                &vec![coin(i * 1_000u128, "uwhale".to_string())],
                |result| {
                    if i > 7 {
                        // this should fail as only 7 incentives can be opened as specified in `instantiate_default`
                        let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                        match err {
                            ContractError::TooManyFlows { .. } => {}
                            _ => panic!(
                                "Wrong error type, should return ContractError::TooManyFlows"
                            ),
                        }
                    } else {
                        result.unwrap();
                    }
                },
            )
            .query_funds(
                incentive_addr.clone().into_inner(),
                AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                |funds| {
                    println!("usdc funds on incentive contract {}: {:?}", i, funds);
                },
            )
            .query_incentive_factory_config(|result| {
                *fee_collector_addr.borrow_mut() = result.unwrap().fee_collector_addr;
            })
            .query_funds(
                fee_collector_addr.clone().into_inner(),
                AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                |funds| {
                    println!("uwhale funds on fee collector{}: {:?}", i, funds);
                },
            )
            .query_funds(
                fee_collector_addr.clone().into_inner(),
                AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                |funds| {
                    println!("usdc funds on fee collector{}: {:?}", i, funds);
                },
            );

        println!("------------");
    }

    // query flows
    let mut incentive_flows = RefCell::new(vec![]);
    suite.query_flows(incentive_addr.clone().into_inner(), |result| {
        let flows = result.unwrap();

        *incentive_flows.borrow_mut() = flows.clone();

        assert_eq!(flows.len(), 7usize);
        assert_eq!(
            flows.first().unwrap(),
            &Flow {
                flow_id: 1,
                flow_creator: carol.clone(),
                flow_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                    amount: Uint128::new(1_000u128),
                },
                claimed_amount: Uint128::zero(),
                curve: Curve::Linear,
                start_timestamp: app_time.clone().seconds(),
                end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
            }
        );
        assert_eq!(
            flows.last().unwrap(),
            &Flow {
                flow_id: 7,
                flow_creator: carol.clone(),
                flow_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string()
                    },
                    amount: Uint128::new(7_000u128),
                },
                claimed_amount: Uint128::zero(),
                curve: Curve::Linear,
                start_timestamp: app_time.clone().seconds(),
                end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
            }
        );
    });

    suite
        .query_flow(incentive_addr.clone().into_inner(), 1u64, |result| {
            let flow_response = result.unwrap();
            assert_eq!(
                flow_response.unwrap().flow,
                Some(Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string()
                        },
                        amount: Uint128::new(1_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                })
            );
        })
        .query_flow(incentive_addr.clone().into_inner(), 5u64, |result| {
            let flow_response = result.unwrap();
            assert_eq!(
                flow_response.unwrap().flow,
                Some(Flow {
                    flow_id: 5,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string()
                        },
                        amount: Uint128::new(5_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                })
            );
        });

    // close 7 incentives, it should fail on the 8th since there are only 7.

    // will err cuz bob didn't create the flow. Flows can only be closed by the creator of the flow
    // or the owner of the contract
    suite.close_incentive_flow(
        bob.clone(),
        incentive_addr.clone().into_inner(),
        1u64,
        |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::UnauthorizedFlowClose { .. } => {}
                _ => panic!("Wrong error type, should return ContractError::UnauthorizedFlowClose"),
            }
        },
    );

    for i in 1..=7 {
        println!("closing flow {}", i);
        let flow = incentive_flows
            .clone()
            .into_inner()
            .get(i - 1)
            .unwrap()
            .clone();

        let sender = if i % 2 == 0 {
            println!("closing flow {} by alice", i);
            alice.clone() // some flows will be closed by the owner of the contract
        } else {
            println!("closing flow {} by carol", i);
            carol.clone() // some flows will be closed by the creator of the flow
        };

        suite.close_incentive_flow(
            sender.clone(),
            incentive_addr.clone().into_inner(),
            flow.flow_id,
            |result| {
                println!("closing flow {} result: {:?}", i, result);
                //result.unwrap();
            },
        );
    }
}

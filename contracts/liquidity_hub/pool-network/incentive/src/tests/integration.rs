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
fn create_incentive_with_duplicate() {
    let mut suite =
        TestingSuite::default_with_balances(coins(1_000_000_000u128, "uwhale".to_string()));
    let creator = suite.creator();
    let unauthorized = suite.senders[2].clone();

    suite.instantiate_default_native_fee().create_lp_tokens();

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
fn try_open_more_flows_than_allowed() {
    let mut suite =
        TestingSuite::default_with_balances(vec![coin(1_000_000_000u128, "uwhale".to_string())]);
    let alice = suite.creator();

    suite.instantiate_default_native_fee().create_lp_tokens();

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
        suite.open_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(i * 2_000u128),
            },
            &vec![coin(i * 2_000u128, "uwhale".to_string())],
            |result| {
                if i > 7 {
                    // this should fail as only 7 incentives can be opened as specified in `instantiate_default`
                    let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                    match err {
                        ContractError::TooManyFlows { .. } => {}
                        _ => panic!("Wrong error type, should return ContractError::TooManyFlows"),
                    }
                } else {
                    result.unwrap();
                }
            },
        );
    }

    let mut incentive_flows = RefCell::new(vec![]);
    suite.query_flows(incentive_addr.clone().into_inner(), |result| {
        let flows = result.unwrap();

        *incentive_flows.borrow_mut() = flows.clone();

        assert_eq!(flows.len(), 7usize);
        assert_eq!(
            flows.first().unwrap(),
            &Flow {
                flow_id: 1,
                flow_creator: alice.clone(),
                flow_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
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
                flow_creator: alice.clone(),
                flow_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::new(13_000u128),
                },
                claimed_amount: Uint128::zero(),
                curve: Curve::Linear,
                start_timestamp: app_time.clone().seconds(),
                end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
            }
        );
    });
}

#[test]
fn try_open_flows_with_wrong_timestamps() {
    todo!();
}

#[test]
fn open_flow_with_fee_native_token_and_flow_same_native_token() {
    let mut suite =
        TestingSuite::default_with_balances(vec![coin(1_000_000_000u128, "uwhale".to_string())]);
    let alice = suite.creator();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_native_fee().create_lp_tokens();

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

    // open incentive flow
    let app_time = suite.get_time();

    suite
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(0u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                // this should fail as not enough funds were sent
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlow { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlow"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                // this should fail as not enough funds were sent to cover for fee + MIN_FLOW_AMOUNT
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::EmptyFlowAfterFee { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlowAfterFee"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(100u128, "uwhale".to_string())],
            |result| {
                // this should fail as not enough funds were sent to cover for fee + MIN_FLOW_AMOUNT
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::EmptyFlowAfterFee { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlowAfterFee"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(2_000u128),
            },
            &vec![coin(500u128, "uwhale".to_string())],
            |result| {
                // this should fail as we didn't send enough funds to cover for the fee
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::FlowFeeNotPaid { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowFeeNotPaid"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(2_000u128),
            },
            &vec![coin(2_000u128, "uwhale".to_string())],
            |result| {
                // this should succeed as we sent enough funds to cover for fee + MIN_FLOW_AMOUNT
                result.unwrap();
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            |funds| {
                // funds on the incentive contract
                assert_eq!(funds, Uint128::new(1_000u128));
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
                // funds on the fee collector
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
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
            // this should not work as there is no flow with id 5
            let flow_response = result.unwrap();
            assert_eq!(flow_response, None);
        });
}

#[test]
fn open_flow_with_fee_native_token_and_flow_different_native_token() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ampWHALE".to_string()),
    ]);
    let alice = suite.creator();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_native_fee().create_lp_tokens();

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

    let mut carol_original_uwhale_funds = RefCell::new(Uint128::zero());

    // open incentive flow
    let app_time = suite.get_time();
    suite
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".clone().to_string(),
                },
                amount: Uint128::new(500u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                // this should fail as MIN_FLOW_AMOUNT is not met
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlow { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlow"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".clone().to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                // this should fail as the flow asset was not sent
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::FlowAssetNotSent { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowAssetNotSent"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".clone().to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![
                coin(1_000u128, "uwhale".to_string()),
                coin(500u128, "ampWHALE".to_string()),
            ],
            |result| {
                // this should fail as the flow asset amount doesn't match the one sent to the contract
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::FlowAssetNotSent { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowAssetNotSent"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".clone().to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![
                coin(100u128, "uwhale".to_string()),
                coin(1_00u128, "ampWHALE".to_string()),
            ],
            |result| {
                // this should fail as not enough funds were sent to cover for fee
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                match err {
                    ContractError::FlowFeeNotPaid { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowFeeNotPaid"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".clone().to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![
                coin(1_000u128, "uwhale".to_string()),
                coin(1_000u128, "ampWHALE".to_string()),
            ],
            |result| {
                // this should succeed as both the fee was paid in full and the flow asset amount
                // matches the one sent to the contract
                result.unwrap();
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            AssetInfo::NativeToken {
                denom: "ampWHALE".to_string(),
            },
            |funds| {
                // funds on the incentive contract
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            |funds| {
                // no uwhale should have been sent to the incentive contract
                assert_eq!(funds, Uint128::zero());
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
                // funds on the fee collector
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_funds(
            fee_collector_addr.clone().into_inner(),
            AssetInfo::NativeToken {
                denom: "ampWHALE".to_string(),
            },
            |funds| {
                // no ampWHALE should have been sent to the fee collector
                assert_eq!(funds, Uint128::zero());
            },
        )
        .query_flow(incentive_addr.clone().into_inner(), 1u64, |result| {
            let flow_response = result.unwrap();
            assert_eq!(
                flow_response.unwrap().flow,
                Some(Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "ampWHALE".to_string()
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
            // this should not work as there is no flow with id 5
            let flow_response = result.unwrap();
            assert_eq!(flow_response, None);
        })
        .query_funds(
            carol.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".clone().to_string(),
            },
            |result| {
                *carol_original_uwhale_funds.borrow_mut() = result;
            },
        )
        // create another incentive overpaying the fee, and check if the excees went back to carol
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".clone().to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![
                coin(50_000u128, "uwhale".to_string()),
                coin(1_000u128, "ampWHALE".to_string()),
            ],
            |result| {
                // this should succeed as we sent enough funds to cover for fee + MIN_FLOW_AMOUNT
                result.unwrap();
            },
        )
        .query_funds(
            carol.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".clone().to_string(),
            },
            |result| {
                // the current balance should be the original minus the fee only, which is 1_000uwhale
                assert_eq!(
                    result,
                    carol_original_uwhale_funds.clone().into_inner() - Uint128::new(1_000u128)
                );
            },
        );
}

#[test]
fn open_flow_with_fee_native_token_and_flow_cw20_token() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ampWHALE".to_string()),
    ]);
    let alice = suite.creator();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_native_fee().create_lp_tokens();

    let mut fee_collector_addr = RefCell::new(Addr::unchecked(""));

    let lp_address_1 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };

    let cw20_incentive = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.last().unwrap().to_string(),
    };

    let cw20_incentive_address = suite.cw20_tokens.last().unwrap().clone();

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

    let mut carol_original_uwhale_funds = RefCell::new(Uint128::zero());

    // open incentive flow
    let app_time = suite.get_time();
    suite
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_incentive.clone(),
                amount: Uint128::new(500u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                // this should fail as MIN_FLOW_AMOUNT is not met
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlow { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlow"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_incentive.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                // this should fail as the flow asset was not sent, i.e. Allowance was not increased
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlowAssetNotSent { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowAssetNotSent"),
                }
            },
        )
        .increase_allowance(
            carol.clone(),
            cw20_incentive_address.clone(),
            Uint128::new(1_000u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_incentive.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                // this should succeed as the allowance was increased
                result.unwrap();
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            cw20_incentive.clone(),
            |funds| {
                // funds on the incentive contract
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            |funds| {
                // no uwhale should have been sent to the incentive contract
                assert_eq!(funds, Uint128::zero());
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
                // funds on the fee collector
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_funds(
            fee_collector_addr.clone().into_inner(),
            cw20_incentive.clone(),
            |funds| {
                // no cw20_incentive amount should have been sent to the fee collector
                assert_eq!(funds, Uint128::zero());
            },
        )
        .query_flow(incentive_addr.clone().into_inner(), 1u64, |result| {
            let flow_response = result.unwrap();
            assert_eq!(
                flow_response.unwrap().flow,
                Some(Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: cw20_incentive.clone(),
                        amount: Uint128::new(1_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                })
            );
        });
}

#[test]
fn open_flow_with_fee_cw20_token_and_flow_same_cw20_token() {
    let mut suite =
        TestingSuite::default_with_balances(vec![coin(1_000_000_000u128, "uwhale".to_string())]);
    let alice = suite.creator();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_cw20_fee().create_lp_tokens();

    let mut fee_collector_addr = RefCell::new(Addr::unchecked(""));

    let lp_address_last = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.last().unwrap().to_string(),
    };

    let cw20_asset = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };

    let cw20_asset_addr = suite.cw20_tokens.first().unwrap().clone();

    let mut incentive_addr = RefCell::new(Addr::unchecked(""));

    suite
        .create_incentive(alice.clone(), lp_address_last.clone(), |result| {
            result.unwrap();
        })
        .query_incentive(lp_address_last.clone(), |result| {
            let incentive = result.unwrap();
            assert!(incentive.is_some());
            *incentive_addr.borrow_mut() = incentive.unwrap();
        });

    // open incentive flow
    let app_time = suite.get_time();

    suite
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(500u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlow { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlow"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent to cover for fee
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlowAfterFee { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlowAfterFee"),
                }
            },
        )
        // let's increase the allowance but not enough to cover for the fees and MIN_FLOW_AMOUNT
        .increase_allowance(
            carol.clone(),
            cw20_asset_addr.clone(),
            Uint128::new(1_500u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(1_500u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent to cover for fee and MIN_FLOW_AMOUNT
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlowAfterFee { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlowAfterFee"),
                }
            },
        )
        .increase_allowance(
            carol.clone(),
            cw20_asset_addr.clone(),
            Uint128::new(2_000u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(2_000u128),
            },
            &vec![],
            |result| {
                // this should succeed as enough funds were sent to cover for fee and MIN_FLOW_AMOUNT
                result.unwrap();
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            cw20_asset.clone(),
            |funds| {
                // funds on the incentive contract
                println!("funds on the incentive contract: {}", funds);
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_incentive_factory_config(|result| {
            *fee_collector_addr.borrow_mut() = result.unwrap().fee_collector_addr;
        })
        .query_funds(
            fee_collector_addr.clone().into_inner(),
            cw20_asset.clone(),
            |funds| {
                // funds on the fee collector
                println!("funds on the fee collector: {}", funds);
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_flow(incentive_addr.clone().into_inner(), 1u64, |result| {
            let flow_response = result.unwrap();
            assert_eq!(
                flow_response.unwrap().flow,
                Some(Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: cw20_asset.clone(),
                        amount: Uint128::new(1_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                })
            );
        });
}

#[test]
fn open_flow_with_fee_cw20_token_and_flow_different_cw20_token() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "ampWHALE".to_string()),
    ]);
    let alice = suite.creator();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_cw20_fee().create_lp_tokens();

    let mut fee_collector_addr = RefCell::new(Addr::unchecked(""));

    let cw20_fee_asset = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };
    let cw20_fee_asset_addr = suite.cw20_tokens.first().unwrap().clone();

    let cw20_asset = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.last().unwrap().to_string(),
    };
    let cw20_asset_addr = suite.cw20_tokens.last().unwrap().clone();

    let mut incentive_addr = RefCell::new(Addr::unchecked(""));

    suite
        .create_incentive(alice.clone(), cw20_fee_asset.clone(), |result| {
            result.unwrap();
        })
        .query_incentive(cw20_fee_asset.clone(), |result| {
            let incentive = result.unwrap();
            assert!(incentive.is_some());
            *incentive_addr.borrow_mut() = incentive.unwrap();
        });

    // open incentive flow
    let app_time = suite.get_time();
    suite
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(500u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlow { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlow"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should fail as the asset to pay for the fee was not transferred
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlowFeeNotPaid { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowFeeNotPaid"),
                }
            },
        )
        // incerase allowance for the flow asset, but not enough to cover the MIN_FLOW_AMOUNT
        .increase_allowance(
            carol.clone(),
            cw20_asset_addr.clone(),
            Uint128::new(500u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent to cover for fee
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlowFeeNotPaid { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowFeeNotPaid"),
                }
            },
        )
        // increase allowance for the fee asset
        .increase_allowance(
            carol.clone(),
            cw20_fee_asset_addr.clone(),
            Uint128::new(1_000u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent to cover the flow asset
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlowAssetNotSent { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowAssetNotSent"),
                }
            },
        )
        // increase allowance for the flow asset
        .increase_allowance(
            carol.clone(),
            cw20_asset_addr.clone(),
            Uint128::new(1_000u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should succeed as both the fee was paid in full and the flow asset amount
                // matches the one sent to the contract
                result.unwrap();
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            cw20_asset.clone(),
            |funds| {
                // funds on the incentive contract
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            cw20_fee_asset.clone(),
            |funds| {
                // no cw20_fee_asset should have been sent to the incentive contract
                assert_eq!(funds, Uint128::zero());
            },
        )
        .query_incentive_factory_config(|result| {
            *fee_collector_addr.borrow_mut() = result.unwrap().fee_collector_addr;
        })
        .query_funds(
            fee_collector_addr.clone().into_inner(),
            cw20_asset.clone(),
            |funds| {
                // no flow assets on the fee collector
                assert_eq!(funds, Uint128::zero());
            },
        )
        .query_funds(
            fee_collector_addr.clone().into_inner(),
            cw20_fee_asset.clone(),
            |funds| {
                // cw20_fee_asset funds on the fee collector
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_flow(incentive_addr.clone().into_inner(), 1u64, |result| {
            let flow_response = result.unwrap();
            assert_eq!(
                flow_response.unwrap().flow,
                Some(Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: cw20_asset.clone(),
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
            // this should not work as there is no flow with id 5
            let flow_response = result.unwrap();
            assert_eq!(flow_response, None);
        });
}

#[test]
fn open_flow_with_fee_cw20_token_and_flow_native_token() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000u128, "uwhale".to_string()),
        coin(1_000_000_000u128, "usdc".to_string()),
    ]);
    let alice = suite.creator();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_cw20_fee().create_lp_tokens();

    let mut fee_collector_addr = RefCell::new(Addr::unchecked(""));

    let cw20_fee_asset = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };
    let cw20_fee_asset_addr = suite.cw20_tokens.first().unwrap().clone();

    let mut incentive_addr = RefCell::new(Addr::unchecked(""));

    suite
        .create_incentive(alice.clone(), cw20_fee_asset.clone(), |result| {
            result.unwrap();
        })
        .query_incentive(cw20_fee_asset.clone(), |result| {
            let incentive = result.unwrap();
            assert!(incentive.is_some());
            *incentive_addr.borrow_mut() = incentive.unwrap();
        });

    let mut carol_original_usdc_funds = RefCell::new(Uint128::zero());

    // open incentive flow
    let app_time = suite.get_time();
    suite
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(500u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::EmptyFlow { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::EmptyFlow"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should fail as the asset to pay for the fee was not transferred
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlowFeeNotPaid { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowFeeNotPaid"),
                }
            },
        )
        // incerase allowance for the fee asset, but not enough
        .increase_allowance(
            carol.clone(),
            cw20_fee_asset_addr.clone(),
            Uint128::new(999u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should fail as not enough funds were sent to cover for fee
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();
                println!("-----");
                match err {
                    ContractError::FlowFeeNotPaid { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowFeeNotPaid"),
                }
            },
        )
        // incerase allowance for the fee asset, enough to cover the fee
        .increase_allowance(
            carol.clone(),
            cw20_fee_asset_addr.clone(),
            Uint128::new(1u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![],
            |result| {
                // this should fail as the flow asset was not sent to the contract
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlowAssetNotSent { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowAssetNotSent"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(900u128, "usdc".to_string())],
            |result| {
                // this should fail as the flow asset was not sent to the contract
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::FlowAssetNotSent { .. } => {}
                    _ => panic!("Wrong error type, should return ContractError::FlowAssetNotSent"),
                }
            },
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "usdc".to_string())],
            |result| {
                // this should succeed as the flow asset was sent to the contract
                result.unwrap();
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            AssetInfo::NativeToken {
                denom: "usdc".to_string(),
            },
            |funds| {
                // funds on the incentive contract
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_funds(
            incentive_addr.clone().into_inner(),
            cw20_fee_asset.clone(),
            |funds| {
                // no cw20_fee_asset should have been sent to the incentive contract
                assert_eq!(funds, Uint128::zero());
            },
        )
        .query_incentive_factory_config(|result| {
            *fee_collector_addr.borrow_mut() = result.unwrap().fee_collector_addr;
        })
        .query_funds(
            fee_collector_addr.clone().into_inner(),
            AssetInfo::NativeToken {
                denom: "usdc".to_string(),
            },
            |funds| {
                // no flow assets on the fee collector
                assert_eq!(funds, Uint128::zero());
            },
        )
        .query_funds(
            fee_collector_addr.clone().into_inner(),
            cw20_fee_asset.clone(),
            |funds| {
                // cw20_fee_asset funds on the fee collector
                assert_eq!(funds, Uint128::new(1_000u128));
            },
        )
        .query_flow(incentive_addr.clone().into_inner(), 1u64, |result| {
            let flow_response = result.unwrap();
            assert_eq!(
                flow_response.unwrap().flow,
                Some(Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "usdc".to_string(),
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
            // this should not work as there is no flow with id 5
            let flow_response = result.unwrap();
            assert_eq!(flow_response, None);
        });
}

#[test]
fn close_native_token_flows() {
    let mut suite =
        TestingSuite::default_with_balances(vec![coin(1_000_000_000u128, "uwhale".to_string())]);
    let alice = suite.creator();
    let bob = suite.senders[1].clone();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_native_fee().create_lp_tokens();

    let mut alice_funds = RefCell::new(Uint128::zero());
    let mut carol_funds = RefCell::new(Uint128::zero());

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

    // open incentive flow
    let app_time = suite.get_time();

    suite
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(2_000u128),
            },
            &vec![coin(2_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .open_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(11_000u128),
            },
            &vec![coin(11_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();

            assert_eq!(flows.len(), 2usize);
            assert_eq!(
                flows.first().unwrap(),
                &Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
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
                    flow_id: 2,
                    flow_creator: alice.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(10_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                }
            );
        })
        .close_incentive_flow(
            bob.clone(),
            incentive_addr.clone().into_inner(),
            1u64,
            |result| {
                // this should error because bob didn't open the flow, nor he is the owner of the incentive
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::UnauthorizedFlowClose { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::UnauthorizedFlowClose"
                    ),
                }
            },
        )
        .close_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            2u64,
            |result| {
                // this should error because carol didn't open the flow, nor he is the owner of the incentive
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::UnauthorizedFlowClose { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::UnauthorizedFlowClose"
                    ),
                }
            },
        )
        .query_funds(
            alice.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            |funds| {
                *alice_funds.borrow_mut() = funds;
            },
        )
        // alice closes her flow
        .close_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            2u64,
            |result| {
                // this should be fine because carol opened the flow
                result.unwrap();
            },
        )
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();

            assert_eq!(flows.len(), 1usize);
            assert_eq!(
                flows.last().unwrap(),
                &Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(1_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                }
            );
        })
        .query_funds(
            alice.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            |funds| {
                // since nothing from the flow was claimed, it means 10_000u128uwhale was returned to alice
                assert_eq!(
                    funds - alice_funds.clone().into_inner(),
                    Uint128::new(10_000u128)
                );
                *alice_funds.borrow_mut() = funds;
            },
        )
        .query_funds(
            carol.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            |funds| {
                *carol_funds.borrow_mut() = funds;
            },
        )
        // alice closes carols flow. She can do it since she is the owner of the flow
        .close_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            1u64,
            |result| {
                result.unwrap();
            },
        )
        .query_funds(
            carol.clone(),
            AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            |funds| {
                // since nothing from the flow was claimed, it means 1_000u128uwhale was returned to carol
                assert_eq!(
                    funds - carol_funds.clone().into_inner(),
                    Uint128::new(1_000u128)
                );
            },
        )
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();
            assert!(flows.is_empty());
        })
        // try closing a flow that doesn't exist
        .close_incentive_flow(
            bob.clone(),
            incentive_addr.clone().into_inner(),
            3u64,
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentFlow { invalid_id } => {
                        assert_eq!(invalid_id, 3u64)
                    }
                    _ => panic!("Wrong error type, should return ContractError::NonExistentFlow"),
                }
            },
        )
        .open_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".clone().to_string(),
                },
                amount: Uint128::new(5_000u128),
            },
            &vec![coin(5_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();

            assert_eq!(flows.len(), 1usize);
            assert_eq!(
                flows.last().unwrap(),
                &Flow {
                    flow_id: 3,
                    flow_creator: alice.clone(),
                    flow_asset: Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(4_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                }
            );
        });
}

#[test]
fn close_cw20_token_flows() {
    let mut suite =
        TestingSuite::default_with_balances(vec![coin(1_000_000_000u128, "uwhale".to_string())]);
    let alice = suite.creator();
    let bob = suite.senders[1].clone();
    let carol = suite.senders[2].clone();

    suite.instantiate_default_native_fee().create_lp_tokens();

    let mut alice_funds = RefCell::new(Uint128::zero());
    let mut carol_funds = RefCell::new(Uint128::zero());

    let lp_address_1 = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.first().unwrap().to_string(),
    };

    let cw20_asset = AssetInfo::Token {
        contract_addr: suite.cw20_tokens.last().unwrap().to_string(),
    };
    let cw20_asset_addr = suite.cw20_tokens.last().unwrap().clone();

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

    // open incentive flow
    let app_time = suite.get_time();

    suite
        .increase_allowance(
            carol.clone(),
            cw20_asset_addr.clone(),
            Uint128::new(1_000u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(1_000u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .increase_allowance(
            alice.clone(),
            cw20_asset_addr.clone(),
            Uint128::new(10_000u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(10_000u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();

            assert_eq!(flows.len(), 2usize);
            assert_eq!(
                flows.first().unwrap(),
                &Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: cw20_asset.clone(),
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
                    flow_id: 2,
                    flow_creator: alice.clone(),
                    flow_asset: Asset {
                        info: cw20_asset.clone(),
                        amount: Uint128::new(10_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                }
            );
        })
        .close_incentive_flow(
            bob.clone(),
            incentive_addr.clone().into_inner(),
            1u64,
            |result| {
                // this should error because bob didn't open the flow, nor he is the owner of the incentive
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::UnauthorizedFlowClose { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::UnauthorizedFlowClose"
                    ),
                }
            },
        )
        .close_incentive_flow(
            carol.clone(),
            incentive_addr.clone().into_inner(),
            2u64,
            |result| {
                // this should error because carol didn't open the flow, nor he is the owner of the incentive
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::UnauthorizedFlowClose { .. } => {}
                    _ => panic!(
                        "Wrong error type, should return ContractError::UnauthorizedFlowClose"
                    ),
                }
            },
        )
        .query_funds(alice.clone(), cw20_asset.clone(), |funds| {
            *alice_funds.borrow_mut() = funds;
        })
        // alice closes her flow
        .close_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            2u64,
            |result| {
                // this should be fine because carol opened the flow
                result.unwrap();
            },
        )
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();

            assert_eq!(flows.len(), 1usize);
            assert_eq!(
                flows.last().unwrap(),
                &Flow {
                    flow_id: 1,
                    flow_creator: carol.clone(),
                    flow_asset: Asset {
                        info: cw20_asset.clone(),
                        amount: Uint128::new(1_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                }
            );
        })
        .query_funds(alice.clone(), cw20_asset.clone(), |funds| {
            // since nothing from the flow was claimed, it means 10_000u128 cw20_asset was returned to alice
            assert_eq!(
                funds - alice_funds.clone().into_inner(),
                Uint128::new(10_000u128)
            );
            *alice_funds.borrow_mut() = funds;
        })
        .query_funds(carol.clone(), cw20_asset.clone(), |funds| {
            *carol_funds.borrow_mut() = funds;
        })
        // alice closes carols flow. She can do it since she is the owner of the flow
        .close_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            1u64,
            |result| {
                result.unwrap();
            },
        )
        .query_funds(carol.clone(), cw20_asset.clone(), |funds| {
            // since nothing from the flow was claimed, it means 1_000u128 cw20_asset was returned to carol
            assert_eq!(
                funds - carol_funds.clone().into_inner(),
                Uint128::new(1_000u128)
            );
        })
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();
            assert!(flows.is_empty());
        })
        // try closing a flow that doesn't exist
        .close_incentive_flow(
            bob.clone(),
            incentive_addr.clone().into_inner(),
            3u64,
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::NonExistentFlow { invalid_id } => {
                        assert_eq!(invalid_id, 3u64)
                    }
                    _ => panic!("Wrong error type, should return ContractError::NonExistentFlow"),
                }
            },
        )
        .increase_allowance(
            alice.clone(),
            cw20_asset_addr.clone(),
            Uint128::new(5_000u128),
            incentive_addr.clone().into_inner(),
        )
        .open_incentive_flow(
            alice.clone(),
            incentive_addr.clone().into_inner(),
            None,
            app_time.clone().plus_seconds(86400u64).seconds(),
            Curve::Linear,
            Asset {
                info: cw20_asset.clone(),
                amount: Uint128::new(5_000u128),
            },
            &vec![coin(1_000u128, "uwhale".to_string())],
            |result| {
                result.unwrap();
            },
        )
        .query_flows(incentive_addr.clone().into_inner(), |result| {
            let flows = result.unwrap();

            assert_eq!(flows.len(), 1usize);
            assert_eq!(
                flows.last().unwrap(),
                &Flow {
                    flow_id: 3,
                    flow_creator: alice.clone(),
                    flow_asset: Asset {
                        info: cw20_asset.clone(),
                        amount: Uint128::new(5_000u128),
                    },
                    claimed_amount: Uint128::zero(),
                    curve: Curve::Linear,
                    start_timestamp: app_time.clone().seconds(),
                    end_timestamp: app_time.clone().plus_seconds(86400u64).seconds(),
                }
            );
        });
}

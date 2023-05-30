use cosmwasm_std::{coins, Event, Uint128};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::whale_lair::WithdrawableResponse;

use crate::tests::robot::TestingRobot;

#[test]
fn test_withdraw_successfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();

    robot
        .instantiate_default()
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(300u128),
            },
            |_res| {},
        )
        .fast_forward(1000u64)
        .assert_withdrawable_response(
            sender.clone().to_string(),
            "ampWHALE".to_string(),
            WithdrawableResponse {
                withdrawable_amount: Uint128::new(300u128),
            },
        )
        .assert_withdrawable_response(
            another_sender.to_string(),
            "ampWHALE".to_string(),
            WithdrawableResponse {
                withdrawable_amount: Uint128::zero(),
            },
        )
        .withdraw(sender.clone(), "ampWHALE".to_string(), |res| {
            let events = res.unwrap().events;
            let transfer_event = events.last().unwrap().clone();
            assert_eq!(
                transfer_event,
                Event::new("transfer").add_attributes(vec![
                    ("recipient", sender.to_string()),
                    ("sender", "contract1".to_string()),
                    ("amount", "300ampWHALE".to_string()),
                ])
            );
        });
}

#[test]
fn test_withdraw_unsuccessfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();

    robot
        .instantiate_default()
        .withdraw(sender.clone(), "ampWHALE".to_string(), |res| {
            println!("{:?}", res.unwrap_err().root_cause());
            //assert error is NothingToWithdraw
        })
        .bond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(300u128),
            },
            |_res| {},
        )
        .withdraw(sender.clone(), "ampWHALE".to_string(), |res| {
            println!("{:?}", res.unwrap_err().root_cause());
            //assert error is NothingToWithdraw
        })
        .fast_forward(999u64) //unbonding period is 1000
        .withdraw(sender.clone(), "ampWHALE".to_string(), |res| {
            println!("{:?}", res.unwrap_err().root_cause());
            //assert error is NothingToWithdraw
        })
        .fast_forward(999u64) //unbonding period is 1000
        .withdraw(sender.clone(), "bWHALE".to_string(), |res| {
            println!("{:?}", res.unwrap_err().root_cause());
            //assert error is NothingToWithdraw
        })
        .withdraw(another_sender, "ampWHALE".to_string(), |res| {
            println!("{:?}", res.unwrap_err().root_cause());
            //assert error is NothingToWithdraw
        });
}

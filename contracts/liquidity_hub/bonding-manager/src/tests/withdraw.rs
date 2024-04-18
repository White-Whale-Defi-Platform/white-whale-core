use cosmwasm_std::{coins, Coin, Event, Uint128};

use white_whale_std::bonding_manager::WithdrawableResponse;

use crate::tests::{bond, robot::TestingRobot};

#[test]
fn test_withdraw_successfully() {
    let mut robot = TestingRobot::default();
    let sender = robot.sender.clone();
    let another_sender = robot.another_sender.clone();

    robot.instantiate_default();

    let bonding_manager_addr = robot.bonding_manager_addr.clone();

    robot
        .bond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
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
        );
    robot.withdraw(sender.clone(), "ampWHALE".to_string(), |res| {
        let events = res.unwrap().events;
        let transfer_event = events.last().unwrap().clone();
        assert_eq!(
            transfer_event,
            Event::new("transfer").add_attributes(vec![
                ("recipient", sender.to_string()),
                ("sender", bonding_manager_addr.to_string()),
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
            Coin {
                denom: "ampWHALE".to_string(),
                amount: Uint128::new(1_000u128),
            },
            &coins(1_000u128, "ampWHALE"),
            |_res| {},
        )
        .fast_forward(10u64)
        .unbond(
            sender.clone(),
            Coin {
                denom: "ampWHALE".to_string(),
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

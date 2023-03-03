use cosmwasm_std::testing::{
    mock_dependencies, mock_dependencies_with_balances, mock_env, mock_info,
};
use cosmwasm_std::{coin, coins, Addr, Uint128};

use white_whale::whale_lair::Config;

use crate::tests::robot::TestingRobot;
use crate::ContractError;

#[test]
fn test_bond() {
    let mut robot = TestingRobot::default();

    /*robot
    .instantiate(
        1_000u64,
        1u8,
        "uwhale".to_string(),
        &vec![coin(1u128, "uwhale")],
    )
    .bond(&coins(1_000u128, "uwhale"), |res| {});*/
}

#[test]
fn test_bond_wrong_asset() {
    let mut robot = TestingRobot::default();

    /*robot
    .instantiate(
        1_000u64,
        1u8,
        "uwhale".to_string(),
        &vec![coin(1u128, "uwhale")],
    )
    .bond(&[coin(1_000u128, "uusdc")], |res| {
        println!("{:?}", res.unwrap_err().root_cause());

        //assert_eq!(res.unwrap_err().root_cause().downcast_ref::<ContractError>().unwrap(), &ContractError::AssetMismatch {});
    })
    .bond(
        &[coin(1_000u128, "uusdc"), coin(1_000u128, "uwhale")],
        |res| {
            println!("{:?}", res.unwrap_err().root_cause());

            //assert_eq!(res.unwrap_err().root_cause().downcast_ref::<ContractError>().unwrap(), &ContractError::AssetMismatch {});
        },
    );*/
}

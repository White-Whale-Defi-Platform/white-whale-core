use cosmwasm_std::{coin, coins, Coin, Decimal, Timestamp, Uint128};
use std::cell::RefCell;

use white_whale_std::bonding_manager::{
    BondedResponse, BondingWeightResponse, GlobalIndex, RewardBucket,
};
use white_whale_std::fee::{Fee, PoolFee};
use white_whale_std::pool_network::asset::MINIMUM_LIQUIDITY_AMOUNT;

use crate::tests::suite::TestingSuite;
use crate::ContractError;

#[test]
fn test_bond_unsuccessful() {
    let mut suite = TestingSuite::default();
    let creator = suite.senders[0].clone();

    suite
        .instantiate_default()
        .bond(creator.clone(), &[], |result| {
            let err = result.unwrap_err().downcast::<ContractError>().unwrap();

            match err {
                ContractError::PaymentError(error) => {
                    assert_eq!(error, cw_utils::PaymentError::NoFunds {})
                }
                _ => panic!("Wrong error type, should return ContractError::PaymentError"),
            }
        })
        .bond(
            creator.clone(),
            &coins(1_000u128, "non_whitelisted_asset"),
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::PaymentError(error) => {
                        assert_eq!(error, cw_utils::PaymentError::NoFunds {})
                    }
                    _ => panic!("Wrong error type, should return PaymentError::NoFunds"),
                }
            },
        )
        .bond(
            creator.clone(),
            &vec![coin(1_000u128, "bWHALE"), coin(1_000u128, "ampWHALE")],
            |result| {
                let err = result.unwrap_err().downcast::<ContractError>().unwrap();

                match err {
                    ContractError::PaymentError(error) => {
                        assert_eq!(error, cw_utils::PaymentError::MultipleDenoms {})
                    }
                    _ => panic!("Wrong error type, should return PaymentError::MultipleDenoms"),
                }
            },
        );
}

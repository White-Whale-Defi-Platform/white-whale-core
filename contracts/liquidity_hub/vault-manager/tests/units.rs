use cosmwasm_std::{Addr, Coin, Uint128};
use white_whale_std::incentive_manager::{Curve, Incentive};

#[test]
fn incentive_expiration() {
    let incentive = Incentive {
        identifier: "identifier".to_string(),
        owner: Addr::unchecked("owner"),
        lp_denom: "lp_denom".to_string(),
        incentive_asset: Coin {
            denom: "asset".to_string(),
            amount: Uint128::new(5_000),
        },
        claimed_amount: Uint128::zero(),
        emission_rate: Uint128::new(1_000),
        curve: Curve::Linear,
        start_epoch: 10,
        preliminary_end_epoch: 14,
        last_epoch_claimed: 9,
    };

    assert!(!incentive.is_expired(9));
    assert!(!incentive.is_expired(12));

    // expired already after 14 days from the last epoch claimed after the incentive started
    assert!(incentive.is_expired(23));
    assert!(incentive.is_expired(33));

    let incentive = Incentive {
        identifier: "identifier".to_string(),
        owner: Addr::unchecked("owner"),
        lp_denom: "lp_denom".to_string(),
        incentive_asset: Coin {
            denom: "asset".to_string(),
            amount: Uint128::new(5_000),
        },
        claimed_amount: Uint128::new(4_001),
        emission_rate: Uint128::new(1_000),
        curve: Curve::Linear,
        start_epoch: 10,
        preliminary_end_epoch: 14,
        last_epoch_claimed: 9,
    };

    // expired already as incentive_asset - claimed is lower than the MIN_INCENTIVE_AMOUNT
    assert!(incentive.is_expired(13));
}

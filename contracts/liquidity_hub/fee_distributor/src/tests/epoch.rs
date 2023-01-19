use cosmwasm_std::Uint128;

use terraswap::asset::{Asset, AssetInfo};

use crate::state::Epoch;

pub(crate) fn get_epochs() -> Vec<Epoch> {
    vec![
        Epoch {
            id: 1,
            total: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(10_000_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    amount: Uint128::from(10_000_000u128),
                },
            ],
            available: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1_000_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    amount: Uint128::from(7_000_000u128),
                },
            ],
            claimed: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(9_000_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    amount: Uint128::from(3_000_000u128),
                },
            ],
        },
        Epoch {
            id: 2,
            total: vec![Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(15_000_000u128),
            }],
            available: vec![Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(3_000_000u128),
            }],
            claimed: vec![Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(12_000_000u128),
            }],
        },
        Epoch {
            id: 2,
            total: vec![Asset {
                info: AssetInfo::NativeToken {
                    denom: "uatom".to_string(),
                },
                amount: Uint128::from(5_000_000u128),
            }],
            available: vec![Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::zero(),
            }],
            claimed: vec![Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::from(5_000_000u128),
            }],
        },
    ]
}

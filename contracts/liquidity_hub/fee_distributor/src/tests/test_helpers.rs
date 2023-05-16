use cosmwasm_std::{Timestamp, Uint128, Uint64};

use white_whale::fee_distributor::Epoch;
use white_whale::pool_network::asset::{Asset, AssetInfo};

pub(crate) fn get_epochs() -> Vec<Epoch> {
    vec![
        Epoch {
            weight: Uint128::from(1_000_000u128),
            id: Uint64::new(1u64),
            start_time: Timestamp::from_seconds(1678726800),
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
            weight: Uint128::from(1_000_000u128),
            id: Uint64::new(2u64),
            start_time: Timestamp::from_seconds(1678813200),
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
                amount: Uint128::from(15_000_000u128),
            }],
            claimed: vec![],
        },
        Epoch {
            weight: Uint128::from(1_000_000u128),
            id: Uint64::new(3u64),
            start_time: Timestamp::from_seconds(1678899600),
            total: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    amount: Uint128::from(5_000_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(5_000_000u128),
                },
            ],
            available: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    amount: Uint128::from(4_000_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(4_000_000u128),
                },
            ],
            claimed: vec![
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    amount: Uint128::from(1_000_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::from(1_000_000u128),
                },
            ],
        },
    ]
}

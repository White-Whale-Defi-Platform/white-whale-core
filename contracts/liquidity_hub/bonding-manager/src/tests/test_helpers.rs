use cosmwasm_std::{Coin, Timestamp, Uint128, Uint64};

use white_whale_std::bonding_manager::Epoch;
use white_whale_std::bonding_manager::GlobalIndex;
use white_whale_std::pool_network::asset::{Asset, AssetInfo};

pub(crate) fn get_epochs() -> Vec<Epoch> {
    vec![
        Epoch {
            global_index: GlobalIndex {
                weight: Uint128::from(1u128),
                bonded_amount: Default::default(),
                bonded_assets: vec![],
                timestamp: Default::default(),
            },
            id: Uint64::new(1u64),
            start_time: Timestamp::from_seconds(1678726800),
            total: vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(10_000_000u128),
                },
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(10_000_000u128),
                },
            ],
            available: vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1_000_000u128),
                },
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(7_000_000u128),
                },
            ],
            claimed: vec![
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(9_000_000u128),
                },
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(3_000_000u128),
                },
            ],
        },
        Epoch {
            global_index: GlobalIndex {
                weight: Uint128::from(1u128),
                bonded_amount: Default::default(),
                bonded_assets: vec![],
                timestamp: Default::default(),
            },
            id: Uint64::new(2u64),
            start_time: Timestamp::from_seconds(1678813200),
            total: vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(15_000_000u128),
            }],
            available: vec![Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::from(15_000_000u128),
            }],
            claimed: vec![],
        },
        Epoch {
            global_index: GlobalIndex {
                weight: Uint128::from(1u128),
                bonded_amount: Default::default(),
                bonded_assets: vec![],
                timestamp: Default::default(),
            },
            id: Uint64::new(3u64),
            start_time: Timestamp::from_seconds(1678899600),
            total: vec![
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(5_000_000u128),
                },
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(5_000_000u128),
                },
            ],
            available: vec![
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(4_000_000u128),
                },
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(4_000_000u128),
                },
            ],
            claimed: vec![
                Coin {
                    denom: "uatom".to_string(),
                    amount: Uint128::from(1_000_000u128),
                },
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::from(1_000_000u128),
                },
            ],
        },
    ]
}

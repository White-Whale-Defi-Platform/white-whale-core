use cosmwasm_std::Uint128;
use white_whale_std::pool_network::asset;

use white_whale_std::pool_network::asset::{Asset, AssetInfo};

#[test]
fn aggregate_fees() {
    let fees: Vec<Asset> = vec![
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(100u128),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "ampWhale".to_string(),
            },
            amount: Uint128::new(500u128),
        },
    ];

    let other_fees: Vec<Asset> = vec![
        Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(200u128),
        },
        Asset {
            info: AssetInfo::NativeToken {
                denom: "bWhale".to_string(),
            },
            amount: Uint128::new(1000u128),
        },
    ];

    let aggregated_fees = asset::aggregate_assets(fees, other_fees).unwrap();

    assert_eq!(
        aggregated_fees,
        vec![
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(300u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWhale".to_string(),
                },
                amount: Uint128::new(500u128),
            },
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "bWhale".to_string(),
                },
                amount: Uint128::new(1000u128),
            },
        ]
    );
}

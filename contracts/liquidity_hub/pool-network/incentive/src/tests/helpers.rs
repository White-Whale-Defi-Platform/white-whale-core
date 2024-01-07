use std::collections::{BTreeMap, HashMap};

use cosmwasm_std::{Addr, Uint128};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::incentive::{Curve, Flow};

use crate::helpers::{get_filtered_flow, get_flow_asset_amount_at_epoch};

#[test]
fn test_get_flow_asset_amount_at_epoch_with_expansion() {
    let mut asset_history = BTreeMap::new();
    asset_history.insert(0, (Uint128::from(10000u128), 105u64));
    asset_history.insert(7, (Uint128::from(20000u128), 110u64));
    asset_history.insert(10, (Uint128::from(50000u128), 115u64));
    let flow = Flow {
        flow_id: 1,
        flow_label: None,
        flow_creator: Addr::unchecked("creator"),
        flow_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::from(10000u128),
        },
        claimed_amount: Uint128::zero(),
        curve: Curve::Linear,
        start_epoch: 0,
        end_epoch: 100,
        emitted_tokens: HashMap::new(),
        asset_history,
    };

    // Before any change
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 0),
        Uint128::from(10000u128)
    );

    // After first change but before second change
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 6),
        Uint128::from(10000u128)
    );
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 7),
        Uint128::from(20000u128)
    );
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 9),
        Uint128::from(20000u128)
    );

    // After second change
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 10),
        Uint128::from(50000u128)
    );
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 11),
        Uint128::from(50000u128)
    );

    // After the end epoch
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 101),
        Uint128::from(50000u128)
    );
}

#[test]
fn test_get_flow_asset_amount_at_epoch_without_expansion() {
    let asset_history = BTreeMap::new();

    let flow = Flow {
        flow_id: 1,
        flow_label: None,
        flow_creator: Addr::unchecked("creator"),
        flow_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::from(10000u128),
        },
        claimed_amount: Uint128::zero(),
        curve: Curve::Linear,
        start_epoch: 0,
        end_epoch: 100,
        emitted_tokens: HashMap::new(),
        asset_history,
    };

    // Before any change
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 0),
        Uint128::from(10000u128)
    );

    // After first change but before second change
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 6),
        Uint128::from(10000u128)
    );
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 7),
        Uint128::from(10000u128)
    );
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 9),
        Uint128::from(10000u128)
    );

    // After second change
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 10),
        Uint128::from(10000u128)
    );
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 11),
        Uint128::from(10000u128)
    );

    // After the end epoch
    assert_eq!(
        get_flow_asset_amount_at_epoch(&flow, 101),
        Uint128::from(10000u128)
    );
}

#[test]
fn get_filtered_flow_cases() {
    let flow = Flow {
        flow_id: 1,
        flow_label: None,
        flow_creator: Addr::unchecked("creator"),
        flow_asset: Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::from(10000u128),
        },
        claimed_amount: Uint128::zero(),
        curve: Curve::Linear,
        start_epoch: 1,
        end_epoch: 100,
        emitted_tokens: HashMap::from_iter((1..105).map(|i| {
            (
                i,
                (Uint128::from(10000u128)
                    .checked_add(Uint128::from(i))
                    .unwrap()),
            )
        })),
        asset_history: BTreeMap::from_iter((1..105).map(|i| {
            (
                i,
                (
                    Uint128::from(10000u128)
                        .checked_add(Uint128::from(i))
                        .unwrap(),
                    105u64,
                ),
            )
        })),
    };

    assert!(flow.asset_history.get(&104).is_some());

    let filtered_flow = get_filtered_flow(flow.clone(), None, None).unwrap();
    assert!(filtered_flow.asset_history.get(&104).is_none());
    assert_eq!(filtered_flow.emitted_tokens.len(), 101usize);
    assert_eq!(filtered_flow.asset_history.len(), 101usize);

    let filtered_flow = get_filtered_flow(flow.clone(), Some(55u64), None).unwrap();
    assert!(filtered_flow.asset_history.get(&54).is_none());
    assert_eq!(filtered_flow.emitted_tokens.len(), 50usize);
    assert_eq!(filtered_flow.asset_history.len(), 50usize);

    let filtered_flow = get_filtered_flow(flow.clone(), Some(110), None).unwrap();
    assert!(filtered_flow.asset_history.is_empty());
    assert!(filtered_flow.emitted_tokens.is_empty());

    let filtered_flow = get_filtered_flow(flow.clone(), Some(11u64), Some(30u64)).unwrap();
    assert!(filtered_flow.asset_history.get(&10).is_none());
    assert!(filtered_flow.emitted_tokens.get(&35).is_none());
    assert_eq!(filtered_flow.emitted_tokens.len(), 20usize);
    assert_eq!(filtered_flow.asset_history.len(), 20usize);

    let filtered_flow = get_filtered_flow(flow, None, Some(50u64)).unwrap();
    assert!(filtered_flow.asset_history.get(&1).is_some());
    assert_eq!(filtered_flow.emitted_tokens.len(), 50usize);
    assert_eq!(filtered_flow.asset_history.len(), 50usize);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::get_flow_asset_amount_at_epoch;
    use cosmwasm_std::{Addr, Uint128};
    use std::collections::{BTreeMap, HashMap};
    use white_whale::pool_network::asset::{Asset, AssetInfo};
    use white_whale::pool_network::incentive::{Curve, Flow};

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
}

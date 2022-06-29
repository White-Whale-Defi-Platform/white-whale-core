use cosmwasm_std::{DepsMut, StdResult};
use terraswap::asset::AssetInfo;
use terraswap::querier::query_token_info;

/// Gets an asset label, used by the factory to create pool pairs and lp tokens with custom names
pub fn get_asset_label(deps: &DepsMut, asset_info: AssetInfo) -> StdResult<String> {
    return match asset_info {
        AssetInfo::Token { contract_addr } => Ok(query_token_info(
            &deps.querier,
            deps.api.addr_validate(contract_addr.as_str())?,
        )?
        .symbol),
        AssetInfo::NativeToken { denom } => Ok(denom),
    };
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
    use cosmwasm_std::Uint128;
    use terraswap::asset::AssetInfo;
    use terraswap::mock_querier::mock_dependencies;

    use crate::asset_helper::get_asset_label;

    #[test]
    fn get_native_asset_label() {
        let mut deps = mock_dependencies(&[]);
        let asset = AssetInfo::NativeToken {
            denom: "native".to_string(),
        };
        let asset_label = get_asset_label(&deps.as_mut(), asset).unwrap();
        assert_eq!(asset_label, "native");
    }

    #[test]
    fn get_token_asset_label() {
        let mut deps = mock_dependencies(&[]);

        deps.querier.with_token_balances(&[(
            &"address".to_string(),
            &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(123u128))],
        )]);

        let asset = AssetInfo::Token {
            contract_addr: "address".to_string(),
        };
        let asset_label = get_asset_label(&deps.as_mut(), asset).unwrap();

        // the Wasm::Smarty query for TokenInfo on the mock_querier returns mAAPL
        assert_eq!(asset_label, "mAAPL");
    }
}

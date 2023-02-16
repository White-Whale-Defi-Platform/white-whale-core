use cosmwasm_std::{to_binary, Binary, Deps, StdError};
use cw_storage_plus::Item;

use pool_network::asset::Asset;
use vault_network::vault::ProtocolFeesResponse;

use crate::error::VaultError;

/// Queries fees on the pool
pub fn get_fees(
    deps: Deps,
    all_time: bool,
    all_time_fees_storage_item: Item<Asset>,
    fees_storage_item: Option<Item<Asset>>,
) -> Result<Binary, VaultError> {
    if all_time {
        let fees = all_time_fees_storage_item.load(deps.storage)?;
        return Ok(to_binary(&ProtocolFeesResponse { fees })?);
    }

    let fees = fees_storage_item
        .ok_or_else(|| StdError::generic_err("fees_storage_item was None"))?
        .load(deps.storage)?;
    Ok(to_binary(&ProtocolFeesResponse { fees })?)
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env},
        Uint128,
    };

    use pool_network::asset::{Asset, AssetInfo};
    use vault_network::vault::{ProtocolFeesResponse, QueryMsg};

    use crate::state::ALL_TIME_BURNED_FEES;
    use crate::{
        contract::query,
        state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES},
    };

    #[test]
    fn returns_fees() {
        let mut deps = mock_dependencies();

        let asset = AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(1_000),
                    info: asset.clone(),
                },
            )
            .unwrap();
        ALL_TIME_COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(5_000),
                    info: asset.clone(),
                },
            )
            .unwrap();

        let res: ProtocolFeesResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ProtocolFees { all_time: false },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            res,
            ProtocolFeesResponse {
                fees: Asset {
                    amount: Uint128::new(1_000),
                    info: asset,
                }
            }
        );
    }

    #[test]
    fn does_allow_all_time() {
        let mut deps = mock_dependencies();

        let asset = AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(1_000),
                    info: asset.clone(),
                },
            )
            .unwrap();
        ALL_TIME_COLLECTED_PROTOCOL_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(5_000),
                    info: asset.clone(),
                },
            )
            .unwrap();

        let res: ProtocolFeesResponse = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::ProtocolFees { all_time: true },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            res,
            ProtocolFeesResponse {
                fees: Asset {
                    amount: Uint128::new(5_000),
                    info: asset,
                }
            }
        );
    }

    #[test]
    fn get_burned_fees() {
        let mut deps = mock_dependencies();

        let asset = AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        ALL_TIME_BURNED_FEES
            .save(
                &mut deps.storage,
                &Asset {
                    amount: Uint128::new(1_000),
                    info: asset.clone(),
                },
            )
            .unwrap();

        let res: ProtocolFeesResponse =
            from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::BurnedFees {}).unwrap())
                .unwrap();
        assert_eq!(
            res,
            ProtocolFeesResponse {
                fees: Asset {
                    amount: Uint128::new(1_000),
                    info: asset,
                }
            }
        );
    }
}

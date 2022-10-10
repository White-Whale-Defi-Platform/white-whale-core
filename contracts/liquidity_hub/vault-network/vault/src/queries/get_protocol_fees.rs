use cosmwasm_std::{to_binary, Binary, Deps};
use vault_network::vault::ProtocolFeesResponse;

use crate::{
    error::VaultError,
    state::{ALL_TIME_COLLECTED_PROTOCOL_FEES, COLLECTED_PROTOCOL_FEES},
};

/// Queries the protocol fees on the pool
pub fn get_protocol_fees(deps: Deps, all_time: bool) -> Result<Binary, VaultError> {
    if all_time {
        let fees = ALL_TIME_COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
        return Ok(to_binary(&ProtocolFeesResponse { fees })?);
    }

    let fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    Ok(to_binary(&ProtocolFeesResponse { fees })?)
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env},
        Uint128,
    };
    use terraswap::asset::{Asset, AssetInfo};
    use vault_network::vault::{ProtocolFeesResponse, QueryMsg};

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
                    info: asset
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
                    info: asset
                }
            }
        );
    }
}

use cosmwasm_std::{Coin, StdError, StdResult, Uint128, Uint64};

use terraswap::asset::{Asset, AssetInfo};

use crate::msg::EpochConfig;
use crate::ContractError;

const MAX_GRACE_PERIOD: u64 = 10u64;
const MAX_EPOCH_START_HOUR: u64 = 23u64;
pub const DAY_IN_SECONDS: u64 = 86400u64;

/// Validates the grace period.
pub fn validate_grace_period(grace_period: &Uint64) -> Result<(), ContractError> {
    if *grace_period < Uint64::one() || *grace_period > Uint64::new(MAX_GRACE_PERIOD) {
        return Err(ContractError::InvalidGracePeriod(*grace_period));
    }

    Ok(())
}

/// Validates the epoch duration.
pub fn validate_epoch_config(epoch_config: &EpochConfig) -> Result<(), ContractError> {
    if epoch_config.duration < Uint64::new(DAY_IN_SECONDS) {
        return Err(ContractError::InvalidEpochDuration(epoch_config.duration));
    }

    Ok(())
}

/// Aggregates assets from two fee vectors, summing up the amounts of assets that are the same.
pub fn aggregate_fees(fees: Vec<Asset>, other_fees: Vec<Asset>) -> Vec<Asset> {
    let mut aggregated_fees = fees;

    for fee in other_fees {
        let mut found = false;
        for aggregated_fee in &mut aggregated_fees {
            if fee.info == aggregated_fee.info {
                aggregated_fee.amount += fee.amount;
                found = true;
                break;
            }
        }

        if !found {
            aggregated_fees.push(fee);
        }
    }

    aggregated_fees
}

/// TODO move this into an impl on pool-network package
/// Converts a vector of Native assets to a vector of coins.
pub fn to_coins(assets: Vec<Asset>) -> StdResult<Vec<Coin>> {
    assets
        .into_iter()
        .map(|asset| {
            let denom = match asset.info {
                AssetInfo::Token { .. } => {
                    return Err(StdError::generic_err("Not a native token."));
                }
                AssetInfo::NativeToken { denom } => denom,
            };

            Ok(Coin {
                denom,
                amount: asset.amount,
            })
        })
        .collect()
}

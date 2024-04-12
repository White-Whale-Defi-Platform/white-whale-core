use cosmwasm_std::{Addr, Coin, Decimal256, Order, StdError, Storage, Uint128};

use white_whale_std::incentive_manager::{Config, EpochId};

use crate::state::LP_WEIGHT_HISTORY;
use crate::ContractError;

const SECONDS_IN_DAY: u64 = 86400;
const SECONDS_IN_YEAR: u64 = 31556926;

/// Calculates the weight size for a user filling a position
pub fn calculate_weight(
    lp_asset: &Coin,
    unlocking_duration: u64,
) -> Result<Uint128, ContractError> {
    if !(SECONDS_IN_DAY..=SECONDS_IN_YEAR).contains(&unlocking_duration) {
        return Err(ContractError::InvalidWeight { unlocking_duration });
    }

    // store in Uint128 form for later
    let amount_uint = lp_asset.amount;

    // interpolate between [(86400, 1), (15778463, 5), (31556926, 16)]
    // note that 31556926 is not exactly one 365-day year, but rather one Earth rotation year
    // similarly, 15778463 is not 1/2 a 365-day year, but rather 1/2 a one Earth rotation year

    // first we need to convert into decimals
    let unlocking_duration = Decimal256::from_atomics(unlocking_duration, 0).unwrap();
    let amount = Decimal256::from_atomics(lp_asset.amount, 0).unwrap();

    let unlocking_duration_squared = unlocking_duration.checked_pow(2)?;
    let unlocking_duration_mul =
        unlocking_duration_squared.checked_mul(Decimal256::raw(109498841))?;
    let unlocking_duration_part =
        unlocking_duration_mul.checked_div(Decimal256::raw(7791996353100889432894))?;

    let next_part = unlocking_duration
        .checked_mul(Decimal256::raw(249042009202369))?
        .checked_div(Decimal256::raw(7791996353100889432894))?;

    let final_part = Decimal256::from_ratio(246210981355969u64, 246918738317569u64);

    let weight: Uint128 = amount
        .checked_mul(
            unlocking_duration_part
                .checked_add(next_part)?
                .checked_add(final_part)?,
        )?
        .atomics()
        .checked_div(10u128.pow(18).into())?
        .try_into()?;

    // we must clamp it to max(computed_value, amount) as
    // otherwise we might get a multiplier of 0.999999999999999998 when
    // computing the final_part decimal value, which is over 200 digits.
    Ok(weight.max(amount_uint))
}

/// Gets the latest available weight snapshot recorded for the given address.
pub fn get_latest_address_weight(
    storage: &dyn Storage,
    address: &Addr,
    lp_denom: &str,
) -> Result<(EpochId, Uint128), ContractError> {
    let result = LP_WEIGHT_HISTORY
        .prefix((address, lp_denom))
        .range(storage, None, None, Order::Descending)
        .take(1usize)
        // take only one item, the last item. Since it's being sorted in descending order, it's the latest one.
        .next()
        .transpose();

    return_latest_weight(result)
}

/// Helper function to return the weight from the result. If the result is None, i.e. the weight
/// was not found in the map, it returns (0, 0).
fn return_latest_weight(
    weight_result: Result<Option<(EpochId, Uint128)>, StdError>,
) -> Result<(EpochId, Uint128), ContractError> {
    match weight_result {
        Ok(Some(item)) => Ok(item),
        Ok(None) => Ok((0u64, Uint128::zero())),
        Err(std_err) => Err(std_err.into()),
    }
}

/// Validates the `unlocking_duration` specified in the position params is within the range specified
/// in the config.
pub(crate) fn validate_unlocking_duration(
    config: &Config,
    unlocking_duration: u64,
) -> Result<(), ContractError> {
    if unlocking_duration < config.min_unlocking_duration
        || unlocking_duration > config.max_unlocking_duration
    {
        return Err(ContractError::InvalidUnlockingDuration {
            min: config.min_unlocking_duration,
            max: config.max_unlocking_duration,
            specified: unlocking_duration,
        });
    }

    Ok(())
}

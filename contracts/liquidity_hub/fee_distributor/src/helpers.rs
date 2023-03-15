use cosmwasm_std::Uint64;

use crate::ContractError;
use white_whale::fee_distributor::EpochConfig;
use white_whale::pool_network::asset::Asset;

const MAX_GRACE_PERIOD: u64 = 10u64;
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

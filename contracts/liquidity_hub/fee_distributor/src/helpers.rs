use cosmwasm_std::Uint64;

use crate::ContractError;
use white_whale_std::epoch_manager::epoch_manager::EpochConfig;

const MAX_GRACE_PERIOD: u64 = 30u64;
pub const DAY_IN_NANOSECONDS: u64 = 86_400_000_000_000u64;

/// Validates the grace period.
pub fn validate_grace_period(grace_period: &Uint64) -> Result<(), ContractError> {
    if *grace_period < Uint64::one() || *grace_period > Uint64::new(MAX_GRACE_PERIOD) {
        return Err(ContractError::InvalidGracePeriod(*grace_period));
    }

    Ok(())
}

/// Validates the epoch duration.
pub fn validate_epoch_config(epoch_config: &EpochConfig) -> Result<(), ContractError> {
    if epoch_config.duration < Uint64::new(DAY_IN_NANOSECONDS) {
        return Err(ContractError::InvalidEpochDuration(epoch_config.duration));
    }

    Ok(())
}

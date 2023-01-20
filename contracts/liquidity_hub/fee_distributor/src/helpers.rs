use crate::ContractError;

/// Validates the grace period.
pub fn validate_grace_period(grace_period: &u128) -> Result<(), ContractError> {
    if *grace_period < 1 || *grace_period > 10 {
        return Err(ContractError::InvalidGracePeriod(*grace_period));
    }

    Ok(())
}

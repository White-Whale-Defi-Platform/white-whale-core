use crate::ContractError;
use terraswap::asset::Asset;

/// Validates the grace period.
pub fn validate_grace_period(grace_period: &u128) -> Result<(), ContractError> {
    if *grace_period < 1 || *grace_period > 10 {
        return Err(ContractError::InvalidGracePeriod(*grace_period));
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

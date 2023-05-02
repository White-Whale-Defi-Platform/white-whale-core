use cosmwasm_std::{Decimal256, Uint128};

use crate::error::ContractError;

/// Calculates the weight size for a user who is creating a position
pub fn calculate_weight(
    unbonding_duration: u64,
    amount: Uint128,
) -> Result<Uint128, ContractError> {
    if unbonding_duration < 86400 || unbonding_duration > 31556926 {
        return Err(ContractError::InvalidWeight { unbonding_duration });
    }

    // store in Uint128 form for later
    let amount_uint = amount;

    // interpolate between [(86400, 1), (15778463, 5), (31556926, 16)]
    // first we need to convert into decimals
    let unbonding_duration = Decimal256::from_atomics(unbonding_duration, 0).unwrap();
    let amount = Decimal256::from_atomics(amount, 0).unwrap();

    let unbonding_duration_squared = unbonding_duration.checked_pow(2)?;
    let unbonding_duration_mul =
        unbonding_duration_squared.checked_mul(Decimal256::raw(109498841))?;
    let unbonding_duration_part =
        unbonding_duration_mul.checked_div(Decimal256::raw(7791996353100889432894))?;

    let next_part = unbonding_duration
        .checked_mul(Decimal256::raw(249042009202369))?
        .checked_div(Decimal256::raw(7791996353100889432894))?;

    let final_part = Decimal256::from_ratio(246210981355969u64, 246918738317569u64);

    let weight: Uint128 = amount
        .checked_mul(
            unbonding_duration_part
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

#[cfg(test)]
mod tests {
    use cosmwasm_std::Uint128;

    use super::calculate_weight;

    #[test]
    fn first_step() {
        let weight = calculate_weight(86400, Uint128::new(10_000))
            .unwrap()
            .u128();
        assert_eq!(weight, 10_000);
    }

    #[test]
    fn second_step() {
        let weight = calculate_weight(31556926, Uint128::new(10_000))
            .unwrap()
            .u128();
        assert_eq!(weight, 159_999);
    }

    #[test]
    fn third_step() {
        let weight = calculate_weight(15778463, Uint128::new(10_000))
            .unwrap()
            .u128();
        assert_eq!(weight, 49_999);
    }

    #[test]
    fn precision_for_million() {
        let weight = calculate_weight(86400, Uint128::new(1_000_000))
            .unwrap()
            .u128();
        assert_eq!(weight, 1_000_000);
    }

    #[test]
    #[should_panic]
    fn does_error_outside_range() {
        // range starts at 86400
        calculate_weight(100, Uint128::new(64)).unwrap();
    }
}

use cosmwasm_std::{Decimal, Decimal256, Fraction, StdError, StdResult, Uint128};
use std::str::FromStr;

/// Default swap slippage in case max_spread is not specified
pub const DEFAULT_SLIPPAGE: &str = "0.01";
/// Cap on the maximum swap slippage that is allowed. If max_spread goes over this limit, it will
/// be capped to this value.
pub const MAX_ALLOWED_SLIPPAGE: &str = "0.5";

/// If `belief_price` and `max_spread` both are given,
/// we compute new spread else we just use pool network
/// spread to check `max_spread`
pub fn assert_max_spread(
    belief_price: Option<Decimal>,
    max_spread: Option<Decimal>,
    offer_amount: Uint128,
    return_amount: Uint128,
    spread_amount: Uint128,
) -> StdResult<()> {
    let max_spread: Decimal256 = max_spread
        .unwrap_or(Decimal::from_str(DEFAULT_SLIPPAGE)?)
        .min(Decimal::from_str(MAX_ALLOWED_SLIPPAGE)?)
        .into();

    if let Some(belief_price) = belief_price {
        let expected_return = offer_amount
            * belief_price
                .inv()
                .ok_or_else(|| StdError::generic_err("Belief price can't be zero"))?;
        let spread_amount = expected_return.saturating_sub(return_amount);

        if return_amount < expected_return
            && Decimal256::from_ratio(spread_amount, expected_return) > max_spread
        {
            return Err(StdError::generic_err("Spread limit exceeded"));
        }
    } else if Decimal256::from_ratio(spread_amount, return_amount + spread_amount) > max_spread {
        return Err(StdError::generic_err("Spread limit exceeded"));
    }

    Ok(())
}

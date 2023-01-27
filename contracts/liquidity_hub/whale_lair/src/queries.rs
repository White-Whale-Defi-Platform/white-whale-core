use cosmwasm_std::{Deps, Order, StdError, StdResult, Uint128};

use white_whale::whale_lair::{Config, Stake, UnstakingResponse};

use crate::state::{CONFIG, STAKE, UNSTAKE};

/// Queries the current configuration of the contract.
pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// Queries the current staked amount of the given address.
pub(crate) fn query_staked(deps: Deps, address: String) -> StdResult<Uint128> {
    let address = deps.api.addr_validate(&address)?;
    let stake = STAKE.may_load(deps.storage, &address)?;

    if let Some(stake) = stake {
        Ok(stake.amount)
    } else {
        Err(StdError::generic_err(format!(
            "No stake found for {}",
            address
        )))
    }
}

pub const MAX_CLAIM_LIMIT: u8 = 30u8;
pub const DEFAULT_CLAIM_LIMIT: u8 = 10u8;

/// Queries the current unstaking amount of the given address.
pub(crate) fn query_unstaking(
    deps: Deps,
    address: String,
    _start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<UnstakingResponse> {
    let address = deps.api.addr_validate(&address)?;

    let limit = limit.unwrap_or(DEFAULT_CLAIM_LIMIT).min(MAX_CLAIM_LIMIT) as usize;
    //let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    let unstaking = UNSTAKE
        .prefix(address.as_bytes())
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, stake) = item?;
            Ok(stake)
        })
        .collect::<StdResult<Vec<Stake>>>()?;

    // aggregate all the amounts in unstaking vec and return uint128
    let unstaking_amount = unstaking.iter().fold(Ok(Uint128::zero()), |acc, stake| {
        acc.and_then(|acc| acc.checked_add(stake.amount))
    })?;

    Ok(UnstakingResponse {
        total_amount: unstaking_amount,
        unstaking_requests: unstaking,
    })
}

/// Queries the amount of unstaking tokens of the specified address that have passed the unstaking period.
pub(crate) fn query_claimable(_deps: Deps, _address: String) -> StdResult<Uint128> {
    unimplemented!()
}

/// Queries the current weight of the given address.
pub(crate) fn query_weight(deps: Deps, address: String) -> StdResult<Uint128> {
    todo!("this is returning the weight that is recorded on the state, but it's not the current one, i.e. it's not updated to the current block height. Maybe this is the desired behavior?");
    let address = deps.api.addr_validate(&address)?;
    let stake = STAKE.may_load(deps.storage, &address)?;
    if let Some(stake) = stake {
        Ok(stake.weight)
    } else {
        Err(StdError::generic_err(format!(
            "No weight found for {}",
            address
        )))
    }
}

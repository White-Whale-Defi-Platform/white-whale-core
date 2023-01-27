use cosmwasm_std::{Deps, Order, StdError, StdResult, Uint128};

use white_whale::whale_lair::{
    ClaimableResponse, Config, GlobalIndex, Stake, StakedResponse, StakingWeightResponse,
    UnstakingResponse,
};

use crate::state::{CONFIG, GLOBAL, STAKE, UNSTAKE};

/// Queries the current configuration of the contract.
pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// Queries the current staked amount of the given address.
pub(crate) fn query_staked(deps: Deps, address: String) -> StdResult<StakedResponse> {
    let address = deps.api.addr_validate(&address)?;
    let stake = STAKE.may_load(deps.storage, &address)?;

    if let Some(stake) = stake {
        Ok(StakedResponse {
            staked: stake.amount,
        })
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
pub(crate) fn query_claimable(
    deps: Deps,
    block_height: u64,
    address: String,
) -> StdResult<ClaimableResponse> {
    let config = CONFIG.load(deps.storage)?;
    let unstaking: StdResult<Vec<_>> = UNSTAKE
        .prefix(address.as_bytes())
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_CLAIM_LIMIT as usize)
        .collect();

    let mut claimable_amount = Uint128::zero();
    for (_, stake) in unstaking? {
        if block_height
            >= stake
                .block_height
                .checked_add(config.unstaking_period)
                .ok_or_else(|| StdError::generic_err("Invalid block height"))?
        {
            claimable_amount += stake.amount;
        }
    }

    Ok(ClaimableResponse { claimable_amount })
}

/// Queries the current weight of the given address.
pub(crate) fn query_weight(
    deps: Deps,
    block_height: u64,
    address: String,
) -> StdResult<StakingWeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let stake = STAKE.may_load(deps.storage, &address)?;

    if let Some(mut stake) = stake {
        let config = CONFIG.load(deps.storage)?;

        stake.weight += stake
            .amount
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                block_height
                    .checked_sub(stake.block_height)
                    .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
            ))?;

        let mut global_index = GLOBAL
            .may_load(deps.storage)
            .unwrap_or_else(|_| Some(GlobalIndex::default()))
            .ok_or_else(|| StdError::generic_err("Global index not found"))?;

        global_index.weight += global_index
            .stake
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                block_height
                    .checked_sub(global_index.block_height)
                    .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
            ))?;

        let share = stake.weight.checked_div(global_index.weight)?;

        Ok(StakingWeightResponse {
            address: address.to_string(),
            weight: stake.weight,
            global_weight: global_index.weight,
            share,
        })
    } else {
        Err(StdError::generic_err(format!(
            "No weight found for {}",
            address
        )))
    }
}

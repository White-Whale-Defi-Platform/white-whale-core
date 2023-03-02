use cosmwasm_std::{Deps, Order, StdError, StdResult, Uint128};

use white_whale::whale_lair::{
    Bond, BondedResponse, BondingWeightResponse, Config, GlobalIndex, UnbondingResponse,
    WithdrawableResponse,
};

use crate::state::{BOND, CONFIG, GLOBAL, UNBOND};

/// Queries the current configuration of the contract.
pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// Queries the current bonded amount of the given address.
pub(crate) fn query_bonded(
    deps: Deps,
    address: String,
    denom: String,
) -> StdResult<BondedResponse> {
    let address = deps.api.addr_validate(&address)?;
    let bond = BOND.may_load(deps.storage, (&address, &denom))?;

    if let Some(bond) = bond {
        Ok(BondedResponse {
            bonded: bond.asset.amount,
        })
    } else {
        Err(StdError::generic_err(format!(
            "No {denom} bond found for {address}"
        )))
    }
}

pub const MAX_CLAIM_LIMIT: u8 = 30u8;
pub const DEFAULT_CLAIM_LIMIT: u8 = 10u8;

/// Queries the current unbonding amount of the given address.
pub(crate) fn query_unbonding(
    deps: Deps,
    address: String,
    denom: String,
    _start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<UnbondingResponse> {
    let address = deps.api.addr_validate(&address)?;

    let limit = limit.unwrap_or(DEFAULT_CLAIM_LIMIT).min(MAX_CLAIM_LIMIT) as usize;
    //let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    let unbonding = UNBOND
        .prefix((&deps.api.addr_validate(address.as_str())?, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, bond) = item?;
            Ok(bond)
        })
        .collect::<StdResult<Vec<Bond>>>()?;

    // aggregate all the amounts in unbonding vec and return uint128
    let unbonding_amount = unbonding.iter().fold(Ok(Uint128::zero()), |acc, bond| {
        acc.and_then(|acc| acc.checked_add(bond.asset.amount))
    })?;

    Ok(UnbondingResponse {
        total_amount: unbonding_amount,
        unbonding_requests: unbonding,
    })
}

/// Queries the amount of unbonding tokens of the specified address that have passed the unbonding period.
pub(crate) fn query_withdrawable(
    deps: Deps,
    block_height: u64,
    address: String,
    denom: String,
) -> StdResult<WithdrawableResponse> {
    let config = CONFIG.load(deps.storage)?;
    let unbonding: StdResult<Vec<_>> = UNBOND
        .prefix((&deps.api.addr_validate(address.as_str())?, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_CLAIM_LIMIT as usize)
        .collect();

    let mut claimable_amount = Uint128::zero();
    for (_, bond) in unbonding? {
        if block_height
            >= bond
                .block_height
                .checked_add(config.unbonding_period)
                .ok_or_else(|| StdError::generic_err("Invalid block height"))?
        {
            claimable_amount = claimable_amount.checked_add(bond.asset.amount)?;
        }
    }

    Ok(WithdrawableResponse { claimable_amount })
}

/// Queries the current weight of the given address.
pub(crate) fn query_weight(
    deps: Deps,
    block_height: u64,
    address: String,
) -> StdResult<BondingWeightResponse> {
    let address = deps.api.addr_validate(&address)?;

    let bonds: StdResult<Vec<_>> = BOND
        .prefix(&address)
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_CLAIM_LIMIT as usize)
        .collect();

    // create bond that will aggregate all the bonds for the given address.
    // This assumes bonding assets are fungible.
    let mut bond = Bond::default();

    for (_, b) in bonds? {
        bond.asset.amount = bond
            .asset
            .amount
            .checked_add(b.asset.amount)
            .expect("error");
        bond.weight = bond.weight.checked_add(b.weight).expect("error");
        bond.block_height = bond
            .block_height
            .checked_add(b.block_height)
            .expect("error");
    }

    let config = CONFIG.load(deps.storage)?;

    bond.weight = bond.weight.checked_add(
        bond.asset
            .amount
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                block_height
                    .checked_sub(bond.block_height)
                    .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
            ))?,
    )?;

    let mut global_index = GLOBAL
        .may_load(deps.storage)
        .unwrap_or_else(|_| Some(GlobalIndex::default()))
        .ok_or_else(|| StdError::generic_err("Global index not found"))?;

    global_index.weight = global_index.weight.checked_add(
        global_index
            .bond_amount
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                block_height
                    .checked_sub(global_index.block_height)
                    .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
            ))?,
    )?;

    let share = bond.weight.checked_div(global_index.weight)?;

    Ok(BondingWeightResponse {
        address: address.to_string(),
        weight: bond.weight,
        global_weight: global_index.weight,
        share,
    })
}

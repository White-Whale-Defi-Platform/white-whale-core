use cosmwasm_std::{Decimal, Deps, Order, StdError, StdResult, Timestamp, Uint128};
use cw_storage_plus::Bound;

use white_whale::whale_lair::{
    Bond, BondedResponse, BondingWeightResponse, Config, GlobalIndex, UnbondingResponse,
    WithdrawableResponse,
};

use crate::state::{get_weight, BOND, BONDING_ASSETS_LIMIT, CONFIG, GLOBAL, UNBOND};

/// Queries the current configuration of the contract.
pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// Queries the current bonded amount of the given address.
pub(crate) fn query_bonded(deps: Deps, address: String) -> StdResult<BondedResponse> {
    let address = deps.api.addr_validate(&address)?;

    let bonds: Vec<Bond> = BOND
        .prefix(&address)
        .range(deps.storage, None, None, Order::Ascending)
        .take(BONDING_ASSETS_LIMIT)
        .map(|item| {
            let (_, bond) = item?;
            Ok(bond)
        })
        .collect::<StdResult<Vec<Bond>>>()?;

    let mut total_bonded = Uint128::zero();
    let mut bonded_assets = vec![];

    for bond in bonds {
        total_bonded = total_bonded.checked_add(bond.asset.amount)?;
        bonded_assets.push(bond.asset);
    }

    Ok(BondedResponse {
        total_bonded,
        bonded_assets,
    })
}

pub const MAX_PAGE_LIMIT: u8 = 30u8;
pub const DEFAULT_PAGE_LIMIT: u8 = 10u8;

/// Queries the current unbonding amount of the given address.
pub(crate) fn query_unbonding(
    deps: Deps,
    address: String,
    denom: String,
    start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<UnbondingResponse> {
    let address = deps.api.addr_validate(&address)?;
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT).min(MAX_PAGE_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    let unbonding = UNBOND
        .prefix((&deps.api.addr_validate(address.as_str())?, &denom))
        .range(deps.storage, start, None, Order::Ascending)
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

fn calc_range_start(start_after: Option<u64>) -> Option<Vec<u8>> {
    start_after.map(|block_height| {
        let mut v: Vec<u8> = block_height.to_be_bytes().to_vec();
        v.push(0);
        v
    })
}

/// Queries the amount of unbonding tokens of the specified address that have passed the
/// unbonding period and can be withdrawn.
pub(crate) fn query_withdrawable(
    deps: Deps,
    timestamp: Timestamp,
    address: String,
    denom: String,
) -> StdResult<WithdrawableResponse> {
    let config = CONFIG.load(deps.storage)?;
    let unbonding: StdResult<Vec<_>> = UNBOND
        .prefix((&deps.api.addr_validate(address.as_str())?, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect();

    let mut withdrawable_amount = Uint128::zero();
    for (_, bond) in unbonding? {
        if timestamp.minus_nanos(config.unbonding_period.u64()) >= bond.timestamp {
            withdrawable_amount = withdrawable_amount.checked_add(bond.asset.amount)?;
        }
    }

    Ok(WithdrawableResponse {
        withdrawable_amount,
    })
}

/// Queries the current weight of the given address.
/// Fable: From the moment a user deposited tokens until now 
/// 
pub(crate) fn query_weight(
    deps: Deps,
    timestamp: Timestamp,
    address: String,
    global_weight: Option<Uint128>,
) -> StdResult<BondingWeightResponse> {
    let address = deps.api.addr_validate(&address)?;

    let bonds: StdResult<Vec<_>> = BOND
        .prefix(&address)
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect();

    let config = CONFIG.load(deps.storage)?;

    let mut total_bond_weight = Uint128::zero();

    for (_, mut bond) in bonds? {
        bond.weight = get_weight(
            timestamp,
            bond.weight,
            bond.asset.amount,
            config.growth_rate,
            bond.timestamp,
        )?;
        println!("bond weight: {}", bond.weight);
        // Aggregate the weights of all the bonds for the given address.
        // This assumes bonding assets are fungible.
        total_bond_weight = total_bond_weight.checked_add(bond.weight)?;
    }
    println!("total bond weight: {}", total_bond_weight);
    // Search bonds for unique bond.asset.denoms 
    // let mut denoms: Vec<String> = bonds?.iter().map(|bond| bond.1.asset.denom.clone()).collect();
    // // // Loop through unbonds and get the ones that are past the unbonding period
    // let unbonding: StdResult<Vec<_>> = UNBOND
    //     .prefix(&address)
    //     .range(deps.storage, None, None, Order::Ascending)
    //     .take(MAX_PAGE_LIMIT as usize)
    //     .collect();

    let mut global_index = GLOBAL
        .may_load(deps.storage)
        .unwrap_or_else(|_| Some(GlobalIndex::default()))
        .ok_or_else(|| StdError::generic_err("Global index not found"))?;
    // If a global weight from an Epoch was passed, use that to get the weight, otherwise use the current global index weight 
    global_index.weight = get_weight(
        timestamp,
        global_weight.unwrap_or_else(||global_index.weight),
        global_index.bonded_amount,
        config.growth_rate,
        global_index.timestamp,
    )?;
    // Represents the share of the global weight that the address has
    let share = Decimal::from_ratio(total_bond_weight, global_index.weight);

    Ok(BondingWeightResponse {
        address: address.to_string(),
        weight: total_bond_weight,
        global_weight: global_index.weight,
        share,
        timestamp,
    })
}

/// Queries the total amount of assets that have been bonded to the contract.
pub fn query_total_bonded(deps: Deps) -> StdResult<BondedResponse> {
    let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    Ok(BondedResponse {
        total_bonded: global_index.bonded_amount,
        bonded_assets: global_index.bonded_assets,
    })
}

/// Queries the global index 
pub fn query_global_index(deps: Deps) -> StdResult<GlobalIndex> {
    let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    println!("global_index: {:?}", global_index);
    Ok(global_index)
}

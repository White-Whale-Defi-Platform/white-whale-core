use std::collections::HashSet;

use cosmwasm_std::{
    to_json_binary, Decimal, Deps, Order, QueryRequest, StdError, StdResult, Timestamp, Uint128,
    Uint64, WasmQuery,
};
use cw_storage_plus::Bound;

use white_whale::fee_distributor::QueryMsg;
use white_whale::{
    pool_network::asset::AssetInfo,
    whale_lair::{
        Bond, BondedResponse, BondingWeightResponse, Config, GlobalIndex, UnbondingResponse,
        WithdrawableResponse,
    },
};

use crate::helpers;
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

    // if it doesn't have bonded, return empty response
    if bonds.is_empty() {
        return Ok(BondedResponse {
            total_bonded: Uint128::zero(),
            bonded_assets: vec![],
            first_bonded_epoch_id: Uint64::zero(),
        });
    }

    let mut total_bonded = Uint128::zero();
    let mut bonded_assets = vec![];

    let mut first_bond_timestamp = Timestamp::from_seconds(16725229261u64);

    for bond in bonds {
        if bond.timestamp.seconds() < first_bond_timestamp.seconds() {
            first_bond_timestamp = bond.timestamp;
        }

        total_bonded = total_bonded.checked_add(bond.asset.amount)?;
        bonded_assets.push(bond.asset);
    }

    let fee_distributor_addr = CONFIG.load(deps.storage)?.fee_distributor_addr;
    let config: white_whale::fee_distributor::Config =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: fee_distributor_addr.to_string(),
            msg: to_json_binary(&QueryMsg::Config {})?,
        }))?;
    let epoch_config = config.epoch_config;
    let first_bonded_epoch_id = helpers::calculate_epoch(epoch_config, first_bond_timestamp)?;

    Ok(BondedResponse {
        total_bonded,
        bonded_assets,
        first_bonded_epoch_id,
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
pub(crate) fn query_weight(
    deps: Deps,
    timestamp: Timestamp,
    address: String,
    global_index: Option<GlobalIndex>,
) -> StdResult<BondingWeightResponse> {
    let address = deps.api.addr_validate(&address)?;

    let bonds: StdResult<Vec<_>> = BOND
        .prefix(&address)
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect();

    let config = CONFIG.load(deps.storage)?;

    let mut total_bond_weight = Uint128::zero();
    // Search bonds for unique bond.asset.denoms
    // Make an empty set of unique denoms
    let mut unique_denoms: HashSet<String> = HashSet::new();

    for (_, mut bond) in bonds? {
        bond.weight = get_weight(
            timestamp,
            bond.weight,
            bond.asset.amount,
            config.growth_rate,
            bond.timestamp,
        )?;

        match bond.asset.info {
            AssetInfo::NativeToken { denom } => {
                // If the denom is not in the set of unique denoms, add it
                if !unique_denoms.contains(&denom) {
                    unique_denoms.insert(denom.clone());
                }
            } //todo the bonding assets can't be Tokens, so this could be removed
            AssetInfo::Token { contract_addr } => {
                // If the contract_addr is not in the set of unique denoms, add it
                if !unique_denoms.contains(&contract_addr) {
                    unique_denoms.insert(contract_addr.clone());
                }
            }
        }
        // Aggregate the weights of all the bonds for the given address.
        // This assumes bonding assets are fungible.
        total_bond_weight = total_bond_weight.checked_add(bond.weight)?;
    }

    let mut global_index = if let Some(global_index) = global_index {
        global_index
    } else {
        GLOBAL
            .may_load(deps.storage)
            .unwrap_or_else(|_| Some(GlobalIndex::default()))
            .ok_or_else(|| StdError::generic_err("Global index not found"))?
    };

    // If a global weight from an Epoch was passed, use that to get the weight, otherwise use the current global index weight
    global_index.weight = get_weight(
        timestamp,
        global_index.weight,
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
        first_bonded_epoch_id: Default::default(), //ignore this parameter here
    })
}

/// Queries the global index
pub fn query_global_index(deps: Deps) -> StdResult<GlobalIndex> {
    let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    Ok(global_index)
}

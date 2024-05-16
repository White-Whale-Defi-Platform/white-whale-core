use std::collections::VecDeque;

use cosmwasm_std::{Decimal, Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use crate::{helpers, ContractError};
use white_whale_std::bonding_manager::{
    Bond, BondedResponse, BondingWeightResponse, Config, GlobalIndex, RewardsResponse,
    UnbondingResponse, WithdrawableResponse,
};
use white_whale_std::bonding_manager::{ClaimableRewardBucketsResponse, RewardBucket};

use crate::state::{
    get_weight, BOND, BONDING_ASSETS_LIMIT, CONFIG, GLOBAL, LAST_CLAIMED_EPOCH, REWARD_BUCKETS,
    UNBOND,
};

/// Queries the current configuration of the contract.
pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// Queries the current bonded amount of the given address. If no address is provided, returns
/// the global bonded amount.
pub(crate) fn query_bonded(deps: Deps, address: Option<String>) -> StdResult<BondedResponse> {
    let (total_bonded, bonded_assets, first_bonded_epoch_id) = if let Some(address) = address {
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
                total_bonded: Default::default(),
                bonded_assets: Default::default(),
                first_bonded_epoch_id: Default::default(),
            });
        }

        let mut total_bonded = Uint128::zero();
        let mut bonded_assets = vec![];
        let mut first_bonded_epoch_id = u64::MAX;

        for bond in bonds {
            if bond.created_at_epoch < first_bonded_epoch_id {
                first_bonded_epoch_id = bond.created_at_epoch;
            }

            total_bonded = total_bonded.checked_add(bond.asset.amount)?;
            bonded_assets.push(bond.asset);
        }

        (total_bonded, bonded_assets, Some(first_bonded_epoch_id))
    } else {
        let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
        (global_index.bonded_amount, global_index.bonded_assets, None)
    };

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
    let unbonding_amount = unbonding.iter().try_fold(Uint128::zero(), |acc, bond| {
        acc.checked_add(bond.asset.amount)
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
    address: String,
    denom: String,
) -> StdResult<WithdrawableResponse> {
    let unbonding: StdResult<Vec<_>> = UNBOND
        .prefix((&deps.api.addr_validate(address.as_str())?, &denom))
        .range(deps.storage, None, None, Order::Ascending)
        .take(MAX_PAGE_LIMIT as usize)
        .collect();

    let config = CONFIG.load(deps.storage)?;
    let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        )?;

    let mut withdrawable_amount = Uint128::zero();
    for (_, bond) in unbonding? {
        if current_epoch.epoch.id.saturating_sub(bond.created_at_epoch) >= config.unbonding_period {
            withdrawable_amount = withdrawable_amount.checked_add(bond.asset.amount)?;
        }
    }

    Ok(WithdrawableResponse {
        withdrawable_amount,
    })
}

/// Queries the current weight of the given address.
pub(crate) fn query_weight(
    deps: &Deps,
    epoch_id: u64,
    address: String,
    mut global_index: GlobalIndex,
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
            epoch_id,
            bond.weight,
            bond.asset.amount,
            config.growth_rate,
            bond.last_updated,
        )?;

        // Aggregate the weights of all the bonds for the given address.
        // This assumes bonding assets are fungible.
        total_bond_weight = total_bond_weight.checked_add(bond.weight)?;
    }

    global_index.last_weight = get_weight(
        epoch_id,
        global_index.last_weight,
        global_index.bonded_amount,
        config.growth_rate,
        global_index.last_updated,
    )?;

    // Represents the share of the global weight that the address has
    // If global_index.weight is zero no one has bonded yet so the share is
    let share = if global_index.last_weight.is_zero() {
        Decimal::zero()
    } else {
        Decimal::from_ratio(total_bond_weight, global_index.last_weight)
    };

    Ok(BondingWeightResponse {
        address: address.to_string(),
        weight: total_bond_weight,
        global_weight: global_index.last_weight,
        share,
        epoch_id,
    })
}

/// Queries the global index
pub fn query_global_index(deps: Deps, reward_bucket_id: Option<u64>) -> StdResult<GlobalIndex> {
    // if a reward_bucket_id is provided, return the global index of the corresponding reward bucket
    if let Some(reward_bucket_id) = reward_bucket_id {
        let reward_bucket = REWARD_BUCKETS.may_load(deps.storage, reward_bucket_id)?;
        return if let Some(reward_bucket) = reward_bucket {
            Ok(reward_bucket.global_index)
        } else {
            Ok(GlobalIndex::default())
        };
    }

    let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
    Ok(global_index)
}

/// Returns the reward bucket that is falling out the grace period, which is the one expiring
/// after creating a new epoch is created.
pub fn get_expiring_reward_bucket(deps: Deps) -> Result<Option<RewardBucket>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let grace_period = config.grace_period;

    // Take grace_period
    let buckets = REWARD_BUCKETS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period as usize)
        .map(|item| {
            let (_, bucket) = item?;
            Ok(bucket)
        })
        .collect::<StdResult<Vec<RewardBucket>>>()?;

    // if the buckets vector's length is the same as the grace period it means there is one bucket that
    // is expiring once the new one is created i.e. the last bucket in the vector
    if buckets.len() == grace_period as usize {
        let expiring_reward_bucket: RewardBucket = buckets.into_iter().last().unwrap_or_default();
        Ok(Some(expiring_reward_bucket))
    } else {
        // nothing is expiring yet
        Ok(None)
    }
}

/// Returns the buckets that are within the grace period, i.e. the ones which fees can still be claimed.
/// The result is ordered by bucket id, descending. Thus, the first element is the current bucket.
pub fn get_claimable_reward_buckets(deps: &Deps) -> StdResult<ClaimableRewardBucketsResponse> {
    let config = CONFIG.load(deps.storage)?;
    let grace_period = config.grace_period;

    let mut reward_buckets = REWARD_BUCKETS
        .range(deps.storage, None, None, Order::Descending)
        .take(grace_period as usize)
        .map(|item| {
            let (_, bucket) = item?;

            Ok(bucket)
        })
        .collect::<StdResult<VecDeque<RewardBucket>>>()?;

    reward_buckets.retain(|bucket| !bucket.available.is_empty());

    Ok(ClaimableRewardBucketsResponse {
        reward_buckets: reward_buckets.into(),
    })
}

/// Returns the reward buckets that can be claimed by the given address. If no address is provided,
/// returns all possible buckets stored in the contract that can potentially be claimed.
pub fn query_claimable(
    deps: &Deps,
    address: Option<String>,
) -> StdResult<ClaimableRewardBucketsResponse> {
    let mut claimable_reward_buckets = get_claimable_reward_buckets(deps)?.reward_buckets;
    // if an address is provided, filter what's claimable for that address
    if let Some(address) = address {
        let address = deps.api.addr_validate(&address)?;

        let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &address)?;

        // filter out buckets that have already been claimed by the user
        if let Some(last_claimed_epoch) = last_claimed_epoch {
            claimable_reward_buckets.retain(|bucket| bucket.id > last_claimed_epoch);
        } else {
            // if the user doesn't have any last_claimed_epoch it means it never bonded
            claimable_reward_buckets.clear();
        };
        // filter out buckets that have no available fees. This would only happen in case the grace period
        // gets increased after buckets have expired, which would lead to make them available for claiming
        // again without any available rewards, as those were forwarded to newer buckets.
        claimable_reward_buckets.retain(|bucket| !bucket.available.is_empty());
    }

    Ok(ClaimableRewardBucketsResponse {
        reward_buckets: claimable_reward_buckets,
    })
}

/// Returns the rewards that can be claimed by the given address.
pub(crate) fn query_rewards(deps: Deps, address: String) -> Result<RewardsResponse, ContractError> {
    let (rewards, _, _) =
        helpers::calculate_rewards(&deps, deps.api.addr_validate(&address)?, false)?;

    Ok(RewardsResponse { rewards })
}

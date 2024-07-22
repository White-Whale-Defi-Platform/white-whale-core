use cosmwasm_std::{Deps, StdResult, Uint128};

use crate::{helpers, ContractError};
use white_whale_std::bonding_manager::ClaimableRewardBucketsResponse;
use white_whale_std::bonding_manager::{
    BondedResponse, Config, GlobalIndex, RewardsResponse, UnbondingResponse, WithdrawableResponse,
};

use crate::state::{
    get_bonds_by_receiver, CONFIG, GLOBAL, LAST_CLAIMED_EPOCH, MAX_LIMIT, REWARD_BUCKETS,
};

/// Queries the current configuration of the contract.
pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

/// Queries the current bonded amount of the given address. If no address is provided, returns
/// the global bonded amount.
pub(crate) fn query_bonded(deps: Deps, address: Option<String>) -> StdResult<BondedResponse> {
    let (total_bonded, bonded_assets) = if let Some(address) = address {
        let address = deps.api.addr_validate(&address)?;

        let bonds = get_bonds_by_receiver(
            deps.storage,
            address.to_string(),
            Some(true),
            None,
            None,
            None,
        )?;

        // if it doesn't have bonded, return empty response
        if bonds.is_empty() {
            return Ok(BondedResponse {
                total_bonded: Default::default(),
                bonded_assets: Default::default(),
            });
        }

        let mut total_bonded = Uint128::zero();
        let mut bonded_assets = vec![];

        for bond in bonds {
            total_bonded = total_bonded.checked_add(bond.asset.amount)?;
            bonded_assets.push(bond.asset);
        }

        (total_bonded, bonded_assets)
    } else {
        let global_index = GLOBAL.may_load(deps.storage)?.unwrap_or_default();
        (global_index.bonded_amount, global_index.bonded_assets)
    };

    Ok(BondedResponse {
        total_bonded,
        bonded_assets,
    })
}

/// Queries the current unbonding amount of the given address.
pub(crate) fn query_unbonding(
    deps: Deps,
    address: String,
    denom: String,
    start_after: Option<u64>,
    limit: Option<u8>,
) -> StdResult<UnbondingResponse> {
    let address = deps.api.addr_validate(&address)?;

    let unbonding = get_bonds_by_receiver(
        deps.storage,
        address.to_string(),
        Some(false),
        Some(denom),
        start_after,
        limit,
    )?;

    // aggregate all the amounts in unbonding vec and return uint128
    let unbonding_amount = unbonding.iter().try_fold(Uint128::zero(), |acc, bond| {
        acc.checked_add(bond.asset.amount)
    })?;

    Ok(UnbondingResponse {
        total_amount: unbonding_amount,
        unbonding_requests: unbonding,
    })
}

/// Queries the amount of unbonding tokens of the specified address that have passed the
/// unbonding period and can be withdrawn.
pub(crate) fn query_withdrawable(
    deps: Deps,
    address: String,
    denom: String,
) -> StdResult<WithdrawableResponse> {
    let unbonding = get_bonds_by_receiver(
        deps.storage,
        address,
        Some(false),
        Some(denom),
        None,
        Some(MAX_LIMIT),
    )?;

    let config = CONFIG.load(deps.storage)?;
    let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        )?;

    let mut withdrawable_amount = Uint128::zero();
    for bond in unbonding {
        if current_epoch.epoch.id.saturating_sub(bond.created_at_epoch) >= config.unbonding_period {
            withdrawable_amount = withdrawable_amount.checked_add(bond.asset.amount)?;
        }
    }

    Ok(WithdrawableResponse {
        withdrawable_amount,
    })
}

/// Queries the global index. If a reward_bucket_id is provided, returns the global index of that reward bucket.
/// Otherwise, returns the current global index.
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

/// Returns the reward buckets that can be claimed by the given address. If no address is provided,
/// returns all possible buckets stored in the contract that can potentially be claimed.
pub fn query_claimable(
    deps: &Deps,
    address: Option<String>,
) -> StdResult<ClaimableRewardBucketsResponse> {
    let mut claimable_reward_buckets = helpers::get_claimable_reward_buckets(deps)?.reward_buckets;
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

    // println!(
    //     ">>> claimable_reward_buckets_2: {:?}",
    //     claimable_reward_buckets
    // );

    Ok(ClaimableRewardBucketsResponse {
        reward_buckets: claimable_reward_buckets,
    })
}

/// Returns the rewards that can be claimed by the given address.
pub(crate) fn query_rewards(deps: Deps, address: String) -> Result<RewardsResponse, ContractError> {
    let (mut rewards, _, _) =
        helpers::calculate_rewards(&deps, deps.api.addr_validate(&address)?, false)?;
    rewards.retain(|coin| coin.amount > Uint128::zero());

    Ok(RewardsResponse { rewards })
}

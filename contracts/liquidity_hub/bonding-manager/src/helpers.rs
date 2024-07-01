use std::collections::{HashMap, VecDeque};

use cosmwasm_std::{
    ensure, to_json_binary, wasm_execute, Addr, Attribute, Coin, CosmosMsg, Decimal, Deps, DepsMut,
    Env, MessageInfo, Order, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, Uint64,
    WasmMsg,
};
use cw_utils::PaymentError;
use serde::Serialize;

use white_whale_std::bonding_manager::ExecuteMsg::ClaimForAddr;
use white_whale_std::bonding_manager::{
    ClaimableRewardBucketsResponse, Config, GlobalIndex, RewardBucket, TemporalBondAction,
};
use white_whale_std::constants::LP_SYMBOL;
use white_whale_std::epoch_manager::epoch_manager::EpochResponse;
use white_whale_std::pool_manager::{
    PoolInfoResponse, SimulateSwapOperationsResponse, SwapRouteResponse,
};
use white_whale_std::pool_network::asset;
use white_whale_std::pool_network::asset::aggregate_coins;

use crate::contract::{LP_WITHDRAWAL_REPLY_ID, NEW_EPOCH_CREATION_REPLY_ID};
use crate::error::ContractError;
use crate::queries::query_claimable;
use crate::state::{
    get_bonds_by_receiver, get_weight, CONFIG, REWARD_BUCKETS, TMP_BOND_ACTION,
    UPCOMING_REWARD_BUCKET,
};

/// Validates that the growth rate is between 0 and 1.
pub fn validate_growth_rate(growth_rate: Decimal) -> Result<(), ContractError> {
    ensure!(
        growth_rate <= Decimal::percent(100),
        ContractError::InvalidGrowthRate
    );
    Ok(())
}

/// Validates that the asset sent on the message matches the asset provided and is whitelisted for bonding.
pub fn validate_funds(deps: &DepsMut, info: &MessageInfo) -> Result<Coin, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    // Ensure that the user has sent some funds
    ensure!(!info.funds.is_empty(), PaymentError::NoFunds {});
    let asset_to_bond = {
        // Filter the funds to include only those with accepted denominations
        let valid_funds: Vec<&Coin> = info
            .funds
            .iter()
            .filter(|coin| config.bonding_assets.contains(&coin.denom))
            .collect();

        // Check if there are no valid funds after filtering
        if valid_funds.is_empty() {
            Err(PaymentError::NoFunds {})
        } else if valid_funds.len() == 1 {
            // If exactly one valid fund is found, return the amount
            Ok(valid_funds[0])
        } else {
            // If multiple valid denominations are found (which shouldn't happen), return an error
            Err(PaymentError::MultipleDenoms {})
        }
    }?;

    Ok(asset_to_bond.to_owned())
}

/// if user has unclaimed rewards, fail with an exception prompting them to claim
pub fn validate_claimed(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    // Do a smart query for Claimable
    let claimable_rewards: ClaimableRewardBucketsResponse =
        query_claimable(&deps.as_ref(), Some(info.sender.to_string()))?;
    // ensure the user has nothing to claim
    ensure!(
        claimable_rewards.reward_buckets.is_empty(),
        ContractError::UnclaimedRewards
    );

    Ok(())
}

/// Validates that the current time is not more than a day after the epoch start time. Helps preventing
/// global_index timestamp issues when querying the weight.
pub fn validate_bonding_for_current_epoch(deps: &DepsMut, env: &Env) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let epoch_response: EpochResponse = deps.querier.query_wasm_smart(
        config.epoch_manager_addr.to_string(),
        &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
    )?;

    let epoch_manager_config: white_whale_std::epoch_manager::epoch_manager::ConfigResponse =
        deps.querier.query_wasm_smart(
            config.epoch_manager_addr.to_string(),
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::Config {},
        )?;

    // Ensure that the current time is not more than the epoch duration after the epoch start time,
    // otherwise it means a new epoch must be created before the user can bond.
    ensure!(
        Uint64::new(env.block.time.nanos())
            .checked_sub(Uint64::new(epoch_response.epoch.start_time.nanos()))?
            < epoch_manager_config.epoch_config.duration,
        ContractError::EpochNotCreatedYet
    );

    let reward_bucket = REWARD_BUCKETS.may_load(deps.storage, epoch_response.epoch.id)?;

    ensure!(reward_bucket.is_some(), ContractError::EpochNotCreatedYet);

    Ok(())
}

// Used in FillRewards to search the funds for LP tokens and withdraw them
// If we do get some LP tokens to withdraw they could be swapped to whale in the reply
pub fn handle_lp_tokens_rewards(
    deps: &DepsMut,
    funds: &[Coin],
    config: &Config,
    submessages: &mut Vec<SubMsg>,
) -> Result<(), ContractError> {
    let lp_tokens: Vec<&Coin> = funds
        .iter()
        .filter(|coin| coin.denom.contains(".pool.") | coin.denom.contains(LP_SYMBOL))
        .collect();

    for lp_token in lp_tokens {
        let pool_identifier =
            extract_pool_identifier(&lp_token.denom).ok_or(ContractError::AssetMismatch)?;

        // make sure a pool with the given identifier exists
        let pool: StdResult<PoolInfoResponse> = deps.querier.query_wasm_smart(
            config.pool_manager_addr.to_string(),
            &white_whale_std::pool_manager::QueryMsg::Pool {
                pool_identifier: pool_identifier.to_string(),
            },
        );

        if pool.is_err() {
            continue;
        }

        // if LP Tokens ,verify and withdraw then swap to whale
        let lp_withdrawal_msg = white_whale_std::pool_manager::ExecuteMsg::WithdrawLiquidity {
            pool_identifier: pool_identifier.to_string(),
        };
        // Add a submessage to withdraw the LP tokens
        let lp_msg: SubMsg = SubMsg {
            id: LP_WITHDRAWAL_REPLY_ID,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.pool_manager_addr.to_string(),
                msg: to_json_binary(&lp_withdrawal_msg)?,
                funds: vec![lp_token.clone()],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        };

        submessages.push(lp_msg);
    }
    Ok(())
}

/// Extracts the pool identifier from an LP token denom.
/// LP tokens have the format "{pair_label}.pool.{identifier}.{LP_SYMBOL}", get the
/// identifier and not the LP SYMBOL. The identifier can contain dots, slashes, etc.
pub(crate) fn extract_pool_identifier(lp_token_denom: &str) -> Option<&str> {
    // Split the string at ".pool." to isolate the part after ".pool."
    let parts: Vec<&str> = lp_token_denom.splitn(2, ".pool.").collect();
    if parts.len() < 2 {
        return None;
    }

    // Split by the last dot to isolate the identifier from "{LP_SYMBOL}"
    let after_pool = parts[1];
    let last_dot_pos = after_pool.rfind('.').unwrap_or(after_pool.len());

    // Take everything before the last dot to get the identifier
    Some(&after_pool[..last_dot_pos])
}

// Used in FillRewards to search the funds for coins that are neither LP tokens nor the distribution_denom
// and swap them to distribution_denom
pub fn swap_coins_to_main_token(
    coins: Vec<Coin>,
    deps: &DepsMut,
    config: Config,
    distribution_asset: &mut Coin,
    distribution_denom: &String,
    messages: &mut Vec<CosmosMsg>,
) -> Result<(), ContractError> {
    let coins_to_swap: Vec<&Coin> = coins
        .iter()
        .filter(|coin| {
            !coin.denom.contains(".pool.")
                & !coin.denom.contains(LP_SYMBOL)
                & !coin.denom.eq(distribution_denom)
        })
        .collect();
    for coin in coins_to_swap {
        // Query for the routes and pool
        let swap_routes_response: StdResult<SwapRouteResponse> = deps.querier.query_wasm_smart(
            config.pool_manager_addr.to_string(),
            &white_whale_std::pool_manager::QueryMsg::SwapRoute {
                offer_asset_denom: coin.denom.to_string(),
                ask_asset_denom: distribution_denom.to_string(),
            },
        );

        let swap_routes = match swap_routes_response {
            Ok(swap_routes) => swap_routes,
            // no routes, skip
            Err(_) => continue,
        };

        // sanity check
        if swap_routes.swap_route.swap_operations.is_empty() {
            // skip swap if there's not swap route for it
            continue;
        }

        // check if the pool has any assets, if not skip the swap
        // Note we are only checking the first operation here.
        // Might be better to another loop to check all operations
        let pool_query = white_whale_std::pool_manager::QueryMsg::Pool {
            pool_identifier: swap_routes
                .swap_route
                .swap_operations
                .first()
                .unwrap()
                .get_pool_identifer(),
        };
        let mut skip_swap = false;
        // Query for the pool to check if it has any assets
        let resp: PoolInfoResponse = deps
            .querier
            .query_wasm_smart(config.pool_manager_addr.to_string(), &pool_query)?;
        // Check pair 'assets' and if either one has 0 amount then don't do swaps
        resp.pool_info.assets.iter().for_each(|asset| {
            if asset.amount.is_zero() {
                skip_swap = true;
            }
        });

        let simulate_swap_operations_response: SimulateSwapOperationsResponse =
            deps.querier.query_wasm_smart(
                config.pool_manager_addr.to_string(),
                &white_whale_std::pool_manager::QueryMsg::SimulateSwapOperations {
                    offer_amount: coin.amount,
                    operations: swap_routes.swap_route.swap_operations.clone(),
                },
            )?;

        // Add the simulate amount received to the distribution_denom amount, if the swap fails this should
        // also be rolled back
        distribution_asset.amount = distribution_asset
            .amount
            .checked_add(simulate_swap_operations_response.amount)?;

        if !skip_swap {
            // Prepare a swap message, use the simulate amount as the minimum receive
            // and 1% slippage to ensure we get at least what was simulated to be received
            let swap_msg = WasmMsg::Execute {
                contract_addr: config.pool_manager_addr.to_string(),
                msg: to_json_binary(
                    &white_whale_std::pool_manager::ExecuteMsg::ExecuteSwapOperations {
                        operations: swap_routes.swap_route.swap_operations.clone(),
                        minimum_receive: Some(simulate_swap_operations_response.amount),
                        receiver: None,
                        max_spread: Some(Decimal::percent(5)),
                    },
                )?,
                funds: vec![coin.clone()],
            };
            messages.push(swap_msg.into());
        }
    }
    Ok(())
}

/// Validates that there are reward buckets in the state. If there are none, it means the system has just
/// been started and the epoch manager has still not created any epochs yet.
pub(crate) fn validate_buckets_not_empty(deps: &DepsMut) -> Result<(), ContractError> {
    let reward_buckets = REWARD_BUCKETS
        .keys(deps.storage, None, None, Order::Descending)
        .collect::<StdResult<Vec<_>>>()?;

    ensure!(!reward_buckets.is_empty(), ContractError::Unauthorized);

    Ok(())
}

type ClaimableRewards = Vec<Coin>;
// key is reward id, value is the rewards claimed from that bucket
type ModifiedRewardBuckets = HashMap<u64, Vec<Coin>>;

/// Calculates the rewards for a user.
pub fn calculate_rewards(
    deps: &Deps,
    address: Addr,
    is_claim: bool,
) -> Result<(ClaimableRewards, Vec<Attribute>, ModifiedRewardBuckets), ContractError> {
    let claimable_reward_buckets_for_user =
        query_claimable(deps, Some(address.to_string()))?.reward_buckets;

    // if the function is being called from the claim function
    if is_claim {
        ensure!(
            !claimable_reward_buckets_for_user.is_empty(),
            ContractError::NothingToClaim
        );
    } else {
        // if the function is being called from the rewards query
        if claimable_reward_buckets_for_user.is_empty() {
            return Ok((vec![], vec![], HashMap::new()));
        }
    }

    let mut total_claimable_rewards = vec![];
    let mut attributes = vec![];
    let mut modified_reward_buckets = HashMap::new();

    for reward_bucket in claimable_reward_buckets_for_user {
        let user_share = get_user_share(
            deps,
            reward_bucket.id,
            address.to_string(),
            reward_bucket.global_index.clone(),
        )?;

        // sanity check, if the user has no share in the bucket, skip it
        if user_share.is_zero() {
            continue;
        };

        // sanity check
        ensure!(
            user_share <= Decimal::percent(100u64),
            ContractError::InvalidShare
        );

        let mut claimed_rewards_from_bucket = vec![];

        for reward in reward_bucket.total.iter() {
            let user_reward = reward.amount.checked_mul_floor(user_share)?;

            // make sure the reward is sound
            let reward_validation: Result<(), StdError> = reward_bucket
                .available
                .iter()
                .find(|available_fee| available_fee.denom == reward.denom)
                .map(|available_reward| {
                    // sanity check
                    if user_reward > available_reward.amount {
                        attributes.push(Attribute {
                            key: "error".to_string(),
                            value: ContractError::InvalidReward {
                                reward: user_reward,
                                available: available_reward.amount,
                            }
                            .to_string(),
                        });
                        return Err(StdError::generic_err("Invalid fee"));
                    }
                    Ok(())
                })
                .ok_or(StdError::generic_err("Invalid fee"))?;

            // if the reward is invalid, skip the bucket
            match reward_validation {
                Ok(_) => {}
                Err(_) => continue,
            }

            let reward = Coin {
                denom: reward.denom.to_string(),
                amount: user_reward,
            };

            // add the reward
            total_claimable_rewards =
                aggregate_coins(&total_claimable_rewards, &vec![reward.clone()])?;

            if is_claim {
                claimed_rewards_from_bucket =
                    aggregate_coins(&claimed_rewards_from_bucket, &vec![reward])?;
                modified_reward_buckets
                    .insert(reward_bucket.id, claimed_rewards_from_bucket.clone());
            }
        }
    }
    Ok((total_claimable_rewards, attributes, modified_reward_buckets))
}

/// Gets the user share for the given epoch and global index.
pub(crate) fn get_user_share(
    deps: &Deps,
    epoch_id: u64,
    address: String,
    mut global_index: GlobalIndex,
) -> StdResult<Decimal> {
    let mut bonds_by_receiver =
        get_bonds_by_receiver(deps.storage, address, Some(true), None, None, None)?;

    let config = CONFIG.load(deps.storage)?;

    let mut total_bond_weight = Uint128::zero();

    for bond in bonds_by_receiver.iter_mut() {
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

    Ok(share)
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

/// Fills the upcoming reward bucket with the given funds.
pub fn fill_upcoming_reward_bucket(deps: DepsMut, funds: Coin) -> StdResult<()> {
    UPCOMING_REWARD_BUCKET.update(deps.storage, |mut upcoming_bucket| -> StdResult<_> {
        upcoming_bucket.total = asset::aggregate_coins(&upcoming_bucket.total, &vec![funds])?;
        Ok(upcoming_bucket)
    })?;

    Ok(())
}

/// Creates a [SubMsg] for the given [TemporalBondAction].
pub fn temporal_bond_action_response(
    deps: &mut DepsMut,
    contract_addr: &Addr,
    temporal_bond_action: TemporalBondAction,
    error: ContractError,
) -> Result<Response, ContractError> {
    TMP_BOND_ACTION.save(deps.storage, &temporal_bond_action)?;

    let submsg = match error {
        ContractError::UnclaimedRewards => create_temporal_bond_action_submsg(
            contract_addr,
            &ClaimForAddr {
                address: temporal_bond_action.sender.to_string(),
            },
        )?,
        ContractError::EpochNotCreatedYet => create_temporal_bond_action_submsg(
            contract_addr,
            &white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::CreateEpoch,
        )?,
        _ => panic!("Can't enter here. Invalid error"),
    };

    Ok(Response::default()
        .add_submessage(submsg)
        .add_attributes(vec![("action", temporal_bond_action.action.to_string())]))
}

/// If there is a new epoch to be created, creates a [SubMsg] to create a new epoch. Used to trigger
/// epoch creation when the user is bonding/unbonding and the epoch has not been created yet.
///
/// If there are unclaimed rewards, creates a [SubMsg] to claim rewards. Used to trigger when the
/// user is bonding/unbonding, and it hasn't claimed pending rewards yet.
fn create_temporal_bond_action_submsg(
    contract_addr: &Addr,
    msg: &impl Serialize,
) -> Result<SubMsg, ContractError> {
    Ok(SubMsg::reply_on_success(
        wasm_execute(contract_addr, msg, vec![])?,
        NEW_EPOCH_CREATION_REPLY_ID,
    ))
}

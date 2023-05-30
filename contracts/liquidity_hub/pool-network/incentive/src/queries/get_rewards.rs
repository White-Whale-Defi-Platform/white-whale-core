use cosmwasm_std::{Decimal256, Deps, Env, Order, StdError, StdResult, Uint128, Uint256};
use white_whale::pool_network::{asset::Asset, incentive::RewardsResponse};
use white_whale::pool_network::incentive::Flow;

use crate::{
    claim::{calculate_claimable_amount, get_user_share},
    state::{FLOWS, LAST_CLAIMED_INDEX},
};
use crate::error::ContractError;
use crate::state::{ADDRESS_WEIGHT_HISTORY, CONFIG, GLOBAL_WEIGHT_SNAPSHOT, LAST_CLAIMED_EPOCH};

pub fn get_rewards(deps: Deps, env: Env, address: String) -> StdResult<RewardsResponse> {
    let address = deps.api.addr_validate(&address)?;

    let config = CONFIG.load(deps.storage)?;
    let epoch_response: white_whale::fee_distributor::EpochResponse =
        deps.querier.query_wasm_smart(
            config.fee_distributor_address.into_string(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )?;
    let current_epoch = epoch_response.epoch.id.u64();


    // let user_share = get_user_share(&deps, address.clone())?;
    //
    // let last_claim_time = LAST_CLAIMED_INDEX
    //     .may_load(deps.storage, address)?
    //     //.unwrap_or(env.block.time.seconds());
    //     .unwrap_or(0u64);

    let mut flows: Vec<Flow> = FLOWS
        .may_load(deps.storage)?
        .unwrap_or_default()
        .into_iter()
        //.filter(|flow| flow.start_timestamp <= env.block.time.seconds())
        .filter(|flow| flow.start_epoch <= current_epoch)
        .collect();

    let mut rewards = vec![];
    for flow in flows.iter() {
        let mut last_user_weight_seen: (u64, Uint128) = (064, Uint128::zero());
        // check what is the earliest available weight for the user
        let first_available_weight_for_user = ADDRESS_WEIGHT_HISTORY
            .prefix(&address.clone())
            .range(deps.storage, None, None, Order::Ascending)
            .take(1)
            .map(|item| Ok(item?))
            .collect::<StdResult<Vec<(u64, Uint128)>>>()?;

        if !first_available_weight_for_user.is_empty() {
            last_user_weight_seen = first_available_weight_for_user[0];
        }

        let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &address.clone())?;
        let first_claimable_epoch = if let Some(last_claimed_epoch) = last_claimed_epoch {
            // start claiming from the last claimed epoch + 1
            last_claimed_epoch + 1
        } else {
            // the user never claimed before
            if flow.start_epoch > last_user_weight_seen.0 {
                // it means the user locked tokens before the flow started. Start from there just to get
                // the ADDRESS_WEIGHT_HISTORY right
                last_user_weight_seen.0
            } else {
                // it means the user locked tokens after the flow started, and last_user_weight_seen.0 has a value
                flow.start_epoch
            }
        };


        let mut total_reward = Uint128::zero();
        for epoch_id in first_claimable_epoch..=current_epoch {
            // get user weight
            let user_weight_at_epoch =
                ADDRESS_WEIGHT_HISTORY.may_load(deps.storage, (&address.clone(), epoch_id))?;

            let user_weight = if let Some(user_weight_at_epoch) = user_weight_at_epoch {
                last_user_weight_seen = (epoch_id, user_weight_at_epoch);
                user_weight_at_epoch
            } else if last_user_weight_seen.0 != 0u64 && last_user_weight_seen.0 <= epoch_id {
                // we have a weight for this user for a previous epoch, but not this one, so we use
                // the last we saw as that's the current one
                last_user_weight_seen.1
            } else {
                // we don't have a weight for this user for this epoch, or any previous epoch, so we
                // skip this epoch. i.e. the user's weight is 0 for this epoch
                continue;
            };

            // check if the flow is active in this epoch
            if epoch_id < flow.start_epoch {
                // the flow is not active at this epoch yet, skip
                continue;
            }

            let global_weight_at_epoch = GLOBAL_WEIGHT_SNAPSHOT
                .may_load(deps.storage, epoch_id)?
                .unwrap_or_default();

            if global_weight_at_epoch == Uint128::zero() {
                // Nothing to compute here as the global weight is 0, we don't want to divide by 0
                continue;
            }

            let user_share_at_epoch = Decimal256::from_ratio(user_weight, global_weight_at_epoch);

            // calculate emissions per epoch
            let emitted_tokens = if flow.emitted_tokens.is_empty() {
                Uint128::zero()
            } else {
                let previous_emission = flow
                    .emitted_tokens
                    .get(&(epoch_id - 1))
                    .unwrap_or(&Uint128::zero())
                    .clone();
                flow.emitted_tokens
                    .get(&epoch_id)
                    .unwrap_or(&previous_emission)
                    .clone()
            };

            // emission = (total_tokens - emitted_tokens_at_epoch) / (flow_start + flow_duration - epoch) = (total_tokens - emitted_tokens_at_epoch) / (flow_end - epoch)
            let emission_per_epoch = flow
                .flow_asset
                .amount
                .saturating_sub(emitted_tokens)
                .checked_div(Uint128::from(flow.end_epoch - epoch_id))?;

            // calculate user reward

            let user_reward = Uint256::from_uint128(emission_per_epoch) * user_share_at_epoch;

            let user_reward_at_epoch: Uint128 = user_reward.try_into()?;

            // sanity check for user_reward_at_epoch
            if user_reward_at_epoch > emission_per_epoch
                || user_reward_at_epoch.checked_add(flow.claimed_amount)? > flow.flow_asset.amount
            {
                return Err(StdError::generic_err("Invalid reward"));
            }

            if user_reward_at_epoch.is_zero() {
                // we don't want to construct a transfer message for the user
                continue;
            }

            total_reward += user_reward_at_epoch;
        }

        rewards.push(Asset {
            amount: total_reward,
            info: flow.flow_asset.info.clone(),
        });

    }
        // let rewards = FLOWS
        // .may_load(deps.storage)?
        // .unwrap_or_default()
        // .into_iter()
        // //.filter(|flow| flow.start_timestamp <= env.block.time.seconds())
        // .filter(|flow| flow.start_epoch <= current_epoch)
        // .map(|flow| {
        //     // let reward = calculate_claimable_amount(&flow, &env, last_claim_time, user_share)?;
        //
        //     let mut last_user_weight_seen: (u64, Uint128) = (064, Uint128::zero());
        //     // check what is the earliest available weight for the user
        //     let first_available_weight_for_user = ADDRESS_WEIGHT_HISTORY
        //         .prefix(&info.sender.clone())
        //         .range(deps.storage, None, None, Order::Ascending)
        //         .take(1)
        //         .map(|item| Ok(item?))
        //         .collect::<StdResult<Vec<(u64, Uint128)>>>()?;
        //
        //     if !first_available_weight_for_user.is_empty() {
        //         last_user_weight_seen = first_available_weight_for_user[0];
        //     }
        //
        //     let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &info.sender.clone())?;
        //     let first_claimable_epoch = if let Some(last_claimed_epoch) = last_claimed_epoch {
        //         // start claiming from the last claimed epoch + 1
        //         last_claimed_epoch + 1
        //     } else {
        //         // the user never claimed before
        //         if flow.start_epoch > last_user_weight_seen.0 {
        //             // it means the user locked tokens before the flow started. Start from there just to get
        //             // the ADDRESS_WEIGHT_HISTORY right
        //             last_user_weight_seen.0
        //         } else {
        //             // it means the user locked tokens after the flow started, and last_user_weight_seen.0 has a value
        //             flow.start_epoch
        //         }
        //     };
        //
        //
        //     let mut total_reward = Uint128::zero();
        //     for epoch_id in first_claimable_epoch..=current_epoch {
        //         // get user weight
        //         let user_weight_at_epoch =
        //             ADDRESS_WEIGHT_HISTORY.may_load(deps.storage, (&info.sender.clone(), epoch_id))?;
        //
        //         let user_weight = if let Some(user_weight_at_epoch) = user_weight_at_epoch {
        //             last_user_weight_seen = (epoch_id, user_weight_at_epoch);
        //             user_weight_at_epoch
        //         } else if last_user_weight_seen.0 != 0u64 && last_user_weight_seen.0 <= epoch_id {
        //             // we have a weight for this user for a previous epoch, but not this one, so we use
        //             // the last we saw as that's the current one
        //             last_user_weight_seen.1
        //         } else {
        //             // we don't have a weight for this user for this epoch, or any previous epoch, so we
        //             // skip this epoch. i.e. the user's weight is 0 for this epoch
        //             continue;
        //         };
        //
        //         // check if the flow is active in this epoch
        //         if epoch_id < flow.start_epoch {
        //             // the flow is not active at this epoch yet, skip
        //             continue;
        //         }
        //
        //         let global_weight_at_epoch = GLOBAL_WEIGHT_SNAPSHOT
        //             .may_load(deps.storage, epoch_id)?
        //             .unwrap_or_default();
        //
        //         if global_weight_at_epoch == Uint128::zero() {
        //             // Nothing to compute here as the global weight is 0, we don't want to divide by 0
        //             continue;
        //         }
        //
        //         let user_share_at_epoch = Decimal256::from_ratio(user_weight, global_weight_at_epoch);
        //
        //         // calculate emissions per epoch
        //         let emitted_tokens = if flow.emitted_tokens.is_empty() {
        //             Uint128::zero()
        //         } else {
        //             let previous_emission = flow
        //                 .emitted_tokens
        //                 .get(&(epoch_id - 1))
        //                 .unwrap_or(&Uint128::zero())
        //                 .clone();
        //             flow.emitted_tokens
        //                 .get(&epoch_id)
        //                 .unwrap_or(&previous_emission)
        //                 .clone()
        //         };
        //
        //         // emission = (total_tokens - emitted_tokens_at_epoch) / (flow_start + flow_duration - epoch) = (total_tokens - emitted_tokens_at_epoch) / (flow_end - epoch)
        //         let emission_per_epoch = flow
        //             .flow_asset
        //             .amount
        //             .saturating_sub(emitted_tokens)
        //             .checked_div(Uint128::from(flow.end_epoch - epoch_id))?;
        //
        //         // calculate user reward
        //
        //         let user_reward = Uint256::from_uint128(emission_per_epoch) * user_share_at_epoch;
        //
        //         let user_reward_at_epoch: Uint128 = user_reward.try_into()?;
        //
        //         // sanity check for user_reward_at_epoch
        //         if user_reward_at_epoch > emission_per_epoch
        //             || user_reward_at_epoch.checked_add(flow.claimed_amount)? > flow.flow_asset.amount
        //         {
        //             return Err(ContractError::InvalidReward {});
        //         }
        //
        //         if user_reward_at_epoch.is_zero() {
        //             // we don't want to construct a transfer message for the user
        //             continue;
        //         }
        //
        //         total_reward += user_reward_at_epoch;
        //     }
        //
        //     Ok(Asset {
        //         amount: total_reward,
        //         info: flow.flow_asset.info,
        //     })
        // })
        // .collect::<StdResult<Vec<_>>>()?;

    Ok(RewardsResponse { rewards })
}

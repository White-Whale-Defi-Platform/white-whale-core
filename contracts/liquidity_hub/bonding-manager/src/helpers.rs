use cosmwasm_std::{
    ensure, to_json_binary, Coin, CosmosMsg, Decimal, DepsMut, Env, MessageInfo, ReplyOn,
    StdResult, SubMsg, Timestamp, Uint64, WasmMsg,
};
use cw_utils::PaymentError;
use white_whale_std::bonding_manager::{ClaimableEpochsResponse, EpochResponse};
use white_whale_std::constants::LP_SYMBOL;
use white_whale_std::epoch_manager::epoch_manager::EpochConfig;
use white_whale_std::pool_manager::{
    PoolInfoResponse, SimulateSwapOperationsResponse, SwapRouteResponse,
};

use crate::contract::LP_WITHDRAWAL_REPLY_ID;
use crate::error::ContractError;
use crate::queries::{get_claimable_epochs, get_current_epoch};
use crate::state::CONFIG;

/// Validates that the growth rate is between 0 and 1.
pub fn validate_growth_rate(growth_rate: Decimal) -> Result<(), ContractError> {
    if growth_rate > Decimal::percent(100) {
        return Err(ContractError::InvalidGrowthRate {});
    }
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
pub fn validate_claimed(deps: &DepsMut, _info: &MessageInfo) -> Result<(), ContractError> {
    // Do a smart query for Claimable
    let claimable_rewards: ClaimableEpochsResponse = get_claimable_epochs(deps.as_ref()).unwrap();
    // If epochs is greater than none
    if !claimable_rewards.epochs.is_empty() {
        return Err(ContractError::UnclaimedRewards {});
    }

    Ok(())
}

/// Validates that the current time is not more than a day after the epoch start time. Helps preventing
/// global_index timestamp issues when querying the weight.
/// global_index timestamp issues when querying the weight.
pub fn validate_bonding_for_current_epoch(deps: &DepsMut, env: &Env) -> Result<(), ContractError> {
    let epoch_response: EpochResponse = get_current_epoch(deps.as_ref()).unwrap();

    let current_epoch = epoch_response.epoch;
    let current_time = env.block.time.seconds();
    const DAY_IN_SECONDS: u64 = 86_400u64;

    // Check if the current time is more than a day after the epoch start time
    // to avoid potential overflow
    if current_epoch.id != Uint64::zero() {
        let start_time_seconds = current_epoch
            .start_time
            .seconds()
            .checked_add(DAY_IN_SECONDS);
        match start_time_seconds {
            Some(start_time_plus_day) => {
                if current_time > start_time_plus_day {
                    return Err(ContractError::NewEpochNotCreatedYet {});
                }
            }
            None => return Err(ContractError::Unauthorized {}),
        }
    }

    Ok(())
}

/// Calculates the epoch id for any given timestamp based on the genesis epoch configuration.
pub fn calculate_epoch(
    genesis_epoch_config: EpochConfig,
    timestamp: Timestamp,
) -> StdResult<Uint64> {
    let epoch_duration: Uint64 = genesis_epoch_config.duration;

    // if this is true, it means the epoch is before the genesis epoch. In that case return Epoch 0.
    if Uint64::new(timestamp.nanos()) < genesis_epoch_config.genesis_epoch {
        return Ok(Uint64::zero());
    }

    let elapsed_time =
        Uint64::new(timestamp.nanos()).checked_sub(genesis_epoch_config.genesis_epoch)?;
    let epoch = elapsed_time
        .checked_div(epoch_duration)?
        .checked_add(Uint64::one())?;

    Ok(epoch)
}

// Used in FillRewards to search the funds for LP tokens and withdraw them
// If we do get some LP tokens to withdraw they could be swapped to whale in the reply
pub fn handle_lp_tokens(
    info: &MessageInfo,
    config: &white_whale_std::bonding_manager::Config,
    submessages: &mut Vec<SubMsg>,
) -> Result<(), ContractError> {
    let lp_tokens: Vec<&Coin> = info
        .funds
        .iter()
        .filter(|coin| coin.denom.contains(".pool.") | coin.denom.contains(LP_SYMBOL))
        .collect();
    for lp_token in lp_tokens {
        // LP tokens have the format "{pair_label}.pool.{identifier}.{LP_SYMBOL}", get the identifier and not the LP SYMBOL
        let pool_identifier = lp_token.denom.split(".pool.").collect::<Vec<&str>>()[1]
            .split('.')
            .collect::<Vec<&str>>()[0];

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

// Used in FillRewards to search the funds for coins that are neither LP tokens nor whale and swap them to whale
pub fn swap_coins_to_main_token(
    coins: Vec<Coin>,
    deps: &DepsMut,
    config: white_whale_std::bonding_manager::Config,
    whale: &mut Coin,
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
        let swap_route_query = white_whale_std::pool_manager::QueryMsg::SwapRoute {
            offer_asset_denom: coin.denom.to_string(),
            ask_asset_denom: distribution_denom.to_string(),
        };

        // Query for the routes and pool
        let swap_routes: SwapRouteResponse = deps
            .querier
            .query_wasm_smart(config.pool_manager_addr.to_string(), &swap_route_query)?;

        ensure!(
            !swap_routes.swap_route.swap_operations.is_empty(),
            ContractError::NoSwapRoute {
                asset1: coin.denom.to_string(),
                asset2: distribution_denom.to_string()
            }
        );
        // check if the pool has any assets, if not skip the swap
        // Note we are only checking the first operation here. Might be better to another loop to check all operations
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

        let simulate: SimulateSwapOperationsResponse = deps.querier.query_wasm_smart(
            config.pool_manager_addr.to_string(),
            &white_whale_std::pool_manager::QueryMsg::SimulateSwapOperations {
                offer_amount: coin.amount,
                operations: swap_routes.swap_route.swap_operations.clone(),
            },
        )?;
        // Add the simulate amount received to the whale amount, if the swap fails this should also be rolled back
        whale.amount = whale.amount.checked_add(simulate.amount)?;

        if !skip_swap {
            // Prepare a swap message, use the simulate amount as the minimum receive
            // and 1% slippage to ensure we get at least what was simulated to be received
            let msg = white_whale_std::pool_manager::ExecuteMsg::ExecuteSwapOperations {
                operations: swap_routes.swap_route.swap_operations.clone(),
                minimum_receive: Some(simulate.amount),
                receiver: None,
                max_spread: Some(Decimal::percent(5)),
            };
            let binary_msg = to_json_binary(&msg)?;
            let wrapped_msg = WasmMsg::Execute {
                contract_addr: config.pool_manager_addr.to_string(),
                msg: binary_msg,
                funds: vec![coin.clone()],
            };
            messages.push(wrapped_msg.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_epoch() {
        let genesis_epoch = EpochConfig {
            duration: Uint64::from(86400000000000u64), // 1 day in nanoseconds
            genesis_epoch: Uint64::from(1683212400000000000u64), // May 4th 2023 15:00:00
        };

        // First bond timestamp equals genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683212400000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(1u64));

        // First bond timestamp is one day after genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683309600000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(2u64));

        // First bond timestamp is three days after genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683471600000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(4u64));

        // First bond timestamp is before genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683212300000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::zero());

        // First bond timestamp is within the same epoch as genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683223200000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(1u64));

        // First bond timestamp is at the end of the genesis epoch, but not exactly (so it's still not epoch 2)
        let first_bond_timestamp = Timestamp::from_nanos(1683298799999999999u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(1u64));

        // First bond timestamp is exactly one nanosecond after the end of an epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683298800000000001u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(2u64));

        // First bond timestamp is June 13th 2023 10:56:53
        let first_bond_timestamp = Timestamp::from_nanos(1686653813000000000u64);
        let epoch = calculate_epoch(genesis_epoch, first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(40u64));
    }
}

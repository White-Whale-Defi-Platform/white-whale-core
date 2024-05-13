use cosmwasm_std::{
    ensure, to_json_binary, Coin, CosmosMsg, Decimal, DepsMut, MessageInfo, Order, ReplyOn,
    StdResult, SubMsg, WasmMsg,
};
use cw_utils::PaymentError;
use white_whale_std::bonding_manager::{ClaimableRewardBucketsResponse, Config};
use white_whale_std::constants::LP_SYMBOL;
use white_whale_std::epoch_manager::epoch_manager::EpochResponse;
use white_whale_std::pool_manager::{
    PoolInfoResponse, SimulateSwapOperationsResponse, SwapRouteResponse,
};

use crate::contract::LP_WITHDRAWAL_REPLY_ID;
use crate::error::ContractError;
use crate::queries::query_claimable;
use crate::state::{CONFIG, REWARD_BUCKETS};

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
        query_claimable(deps.as_ref(), Some(info.sender.to_string())).unwrap();
    // ensure the user has nothing to claim
    ensure!(
        claimable_rewards.reward_buckets.is_empty(),
        ContractError::UnclaimedRewards
    );

    Ok(())
}

/// Validates that the current time is not more than a day after the epoch start time. Helps preventing
/// global_index timestamp issues when querying the weight.
pub fn validate_bonding_for_current_epoch(deps: &DepsMut) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let epoch_response: EpochResponse = deps.querier.query_wasm_smart(
        config.epoch_manager_addr.to_string(),
        &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
    )?;

    let reward_bucket = REWARD_BUCKETS.may_load(deps.storage, epoch_response.epoch.id)?;

    ensure!(reward_bucket.is_some(), ContractError::EpochNotCreatedYet);

    Ok(())
}

// Used in FillRewards to search the funds for LP tokens and withdraw them
// If we do get some LP tokens to withdraw they could be swapped to whale in the reply
pub fn handle_lp_tokens(
    funds: &Vec<Coin>,
    config: &Config,
    submessages: &mut Vec<SubMsg>,
) -> Result<(), ContractError> {
    println!("funds: {:?}", funds);
    let lp_tokens: Vec<&Coin> = funds
        .iter()
        .filter(|coin| coin.denom.contains(".pool.") | coin.denom.contains(LP_SYMBOL))
        .collect();

    println!("lp_tokens: {:?}", lp_tokens);

    for lp_token in lp_tokens {
        let pool_identifier =
            extract_pool_identifier(&lp_token.denom).ok_or(ContractError::AssetMismatch)?;

        println!("pool_identifier: {:?}", pool_identifier);

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
fn extract_pool_identifier(lp_token_denom: &str) -> Option<&str> {
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
    to_be_distribution_asset: &mut Coin,
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
        println!("Swapping {} to {}", coin.denom, distribution_denom);

        let swap_route_query = white_whale_std::pool_manager::QueryMsg::SwapRoute {
            offer_asset_denom: coin.denom.to_string(),
            ask_asset_denom: distribution_denom.to_string(),
        };

        println!("he");
        // Query for the routes and pool
        let swap_routes_response: StdResult<SwapRouteResponse> = deps
            .querier
            .query_wasm_smart(config.pool_manager_addr.to_string(), &swap_route_query);

        println!("swap_routes_response: {:?}", swap_routes_response);
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
        to_be_distribution_asset.amount = to_be_distribution_asset
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
pub(crate) fn validate_buckets(deps: &DepsMut) -> Result<(), ContractError> {
    let reward_buckets = REWARD_BUCKETS
        .keys(deps.storage, None, None, Order::Descending)
        .collect::<StdResult<Vec<_>>>()?;

    ensure!(!reward_buckets.is_empty(), ContractError::Unauthorized);

    Ok(())
}

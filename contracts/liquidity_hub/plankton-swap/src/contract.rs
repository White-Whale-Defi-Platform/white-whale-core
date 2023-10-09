#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use white_whale::pool_network::pair::FeatureToggle;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::queries::{get_swap_route, get_swap_routes};
use crate::state::{Config, MANAGER_CONFIG};
use crate::{commands, queries};
/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:plankton-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config: Config = Config {
        fee_collector_addr: deps.api.addr_validate(&msg.fee_collector_addr)?,
        owner: deps.api.addr_validate(&msg.owner)?,
        token_code_id: msg.token_code_id,
        // We must set a creation fee on instantiation to prevent spamming of pools
        pool_creation_fee: msg.pool_creation_fee,
        feature_toggle: FeatureToggle {
            withdrawals_enabled: true,
            deposits_enabled: true,
            swaps_enabled: true,
        },
    };
    MANAGER_CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreatePair {
            asset_infos,
            pool_fees,
            pair_type,
            token_factory_lp,
        } => commands::create_pair(
            deps,
            env,
            info,
            asset_infos,
            pool_fees,
            pair_type,
            token_factory_lp,
        ),
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            receiver,
        } => commands::liquidity::provide_liquidity(
            deps,
            env,
            info,
            assets,
            slippage_tolerance,
            receiver,
        ),
        ExecuteMsg::Swap {
            offer_asset,
            ask_asset,
            belief_price,
            max_spread,
            to,
        } => {
            // check if the swap feature is enabled
            let feature_toggle: FeatureToggle = MANAGER_CONFIG.load(deps.storage)?.feature_toggle;
            if !feature_toggle.swaps_enabled {
                return Err(ContractError::OperationDisabled("swap".to_string()));
            }

            if !offer_asset.is_native_token() {
                return Err(ContractError::Unauthorized {});
            }

            let to_addr = if let Some(to_addr) = to {
                Some(deps.api.addr_validate(&to_addr)?)
            } else {
                None
            };
            commands::swap::swap(
                deps,
                env,
                info.clone(),
                info.sender,
                offer_asset,
                ask_asset,
                belief_price,
                max_spread,
                to_addr,
            )
        }
        ExecuteMsg::WithdrawLiquidity { assets } => commands::liquidity::withdraw_liquidity(
            deps,
            env,
            info.sender,
            info.funds[0].amount,
            assets,
        ),
        ExecuteMsg::AddNativeTokenDecimals { denom, decimals } => {
            commands::add_native_token_decimals(deps, env, denom, decimals)
        }
        // ExecuteMsg::UpdatePairInfo { pair_key } => {
        //     commands::update_pair_info(deps, env, denom, decimals)
        // },
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, _msg: QueryMsg) -> Result<Binary, ContractError> {
    match _msg {
        QueryMsg::NativeTokenDecimals { denom } => Ok(to_binary(
            &queries::query_native_token_decimal(deps, denom)?,
        )?),
        QueryMsg::Simulation {
            offer_asset,
            ask_asset,
        } => Ok(to_binary(&queries::query_simulation(
            deps,
            env,
            offer_asset,
            ask_asset,
        )?)?),
        QueryMsg::ReverseSimulation {
            ask_asset,
            offer_asset,
        } => Ok(to_binary(&queries::query_reverse_simulation(
            deps,
            env,
            ask_asset,
            offer_asset,
        )?)?),
        // QueryMsg::ProtocolFees { asset_id, all_time } => Ok(to_binary(&queries::query_fees(
        //     deps,
        //     asset_id,
        //     all_time,
        //     COLLECTED_PROTOCOL_FEES,
        //     Some(ALL_TIME_COLLECTED_PROTOCOL_FEES),
        // )?)?),
        // QueryMsg::BurnedFees { asset_id } => Ok(to_binary(&queries::query_fees(
        //     deps,
        //     asset_id,
        //     None,
        //     ALL_TIME_BURNED_FEES,
        //     None,
        // )?)?),
        // QueryMsg::SimulateSwapOperations {
        //     offer_amount,
        //     operations,
        // } => Ok(to_binary(&simulate_swap_operations(
        //     deps,
        //     offer_amount,
        //     operations,
        // )?)?),
        // QueryMsg::ReverseSimulateSwapOperations {
        //     ask_amount,
        //     operations,
        // } => Ok(to_binary(&reverse_simulate_swap_operations(
        //     deps, ask_amount, operations,
        // )?)?),
        QueryMsg::SwapRoute {
            offer_asset_info,
            ask_asset_info,
        } => Ok(to_binary(&get_swap_route(
            deps,
            offer_asset_info,
            ask_asset_info,
        )?)?),
        QueryMsg::SwapRoutes {} => Ok(to_binary(&get_swap_routes(deps)?)?),
    }
}

#[cfg(test)]
mod tests {}

use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, Uint128,
};
use cw20::Cw20ReceiveMsg;
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::pair::{self, FeatureToggle};
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::queries::{get_swap_route, get_swap_routes};
use crate::state::{Config, MANAGER_CONFIG, PAIRS, PAIR_COUNTER};
use crate::{liquidity, manager, queries, swap};
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
    // initialize vault counter
    PAIR_COUNTER.save(deps.storage, &0u64)?;

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
            pair_identifier,
        } => manager::commands::create_pair(
            deps,
            env,
            info,
            asset_infos,
            pool_fees,
            pair_type,
            token_factory_lp,
            pair_identifier,
        ),
        ExecuteMsg::ProvideLiquidity {
            assets,
            slippage_tolerance,
            receiver,
            pair_identifier,
        } => liquidity::commands::provide_liquidity(
            deps,
            env,
            info,
            assets,
            slippage_tolerance,
            receiver,
            pair_identifier,
        ),
        ExecuteMsg::Swap {
            offer_asset,
            ask_asset,
            belief_price,
            max_spread,
            to,
            pair_identifier,
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
            swap::commands::swap(
                deps,
                env,
                info.clone(),
                info.sender,
                offer_asset,
                ask_asset,
                belief_price,
                max_spread,
                to_addr,
                pair_identifier,
            )
        }
        ExecuteMsg::WithdrawLiquidity {
            assets,
            pair_identifier,
        } => liquidity::commands::withdraw_liquidity(
            deps,
            env,
            info.sender,
            info.funds[0].amount,
            pair_identifier,
        ),
        ExecuteMsg::AddNativeTokenDecimals { denom, decimals } => {
            manager::commands::add_native_token_decimals(deps, env, denom, decimals)
        }
        ExecuteMsg::UpdateOwnership(action) => {
            Ok(
                cw_ownable::update_ownership(deps, &env.block, &info.sender, action).map(
                    |ownership| {
                        Response::default()
                            .add_attribute("action", "update_ownership")
                            .add_attributes(ownership.into_attributes())
                    },
                )?,
            )
        }
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
    }
}

#[cw_serde]
pub enum Cw20HookMsg {
    /// Sell a given amount of asset
    Swap {
        ask_asset: AssetInfo,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        pair_identifier: String,
    },
    /// Withdraws liquidity
    WithdrawLiquidity { pair_identifier: String },
}

/// Receives cw20 tokens. Used to swap and withdraw from the pool.
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let contract_addr = info.sender.clone();
    let feature_toggle: FeatureToggle = MANAGER_CONFIG.load(deps.storage)?.feature_toggle;

    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Swap {
            ask_asset,
            belief_price,
            max_spread,
            to,
            pair_identifier,
        }) => {
            // check if the swap feature is enabled
            if !feature_toggle.swaps_enabled {
                return Err(ContractError::OperationDisabled("swap".to_string()));
            }

            let to_addr = if let Some(to_addr) = to {
                Some(deps.api.addr_validate(to_addr.as_str())?)
            } else {
                None
            };

            crate::swap::commands::swap(
                deps,
                env,
                info,
                Addr::unchecked(cw20_msg.sender),
                Asset {
                    info: AssetInfo::Token {
                        contract_addr: contract_addr.to_string(),
                    },
                    amount: cw20_msg.amount,
                },
                ask_asset,
                belief_price,
                max_spread,
                to_addr,
                pair_identifier,
            )
        }
        Ok(Cw20HookMsg::WithdrawLiquidity { pair_identifier }) => {
            // check if the withdrawal feature is enabled
            if !feature_toggle.withdrawals_enabled {
                return Err(ContractError::OperationDisabled(
                    "withdraw_liquidity".to_string(),
                ));
            }

            let sender_addr = deps.api.addr_validate(cw20_msg.sender.as_str())?;
            crate::liquidity::commands::withdraw_liquidity(
                deps,
                env,
                sender_addr,
                cw20_msg.amount,
                pair_identifier,
            )
        }
        Err(err) => Err(ContractError::Std(err)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
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
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations,
        } => Ok(to_binary(&queries::simulate_swap_operations(
            deps,
            env,
            offer_amount,
            operations,
        )?)?),
        QueryMsg::ReverseSimulateSwapOperations {
            ask_amount,
            operations,
        } => Ok(to_binary(&queries::reverse_simulate_swap_operations(
            deps, env, ask_amount, operations,
        )?)?),
        QueryMsg::SwapRoute {
            offer_asset_info,
            ask_asset_info,
        } => Ok(to_binary(&get_swap_route(
            deps,
            offer_asset_info,
            ask_asset_info,
        )?)?),
        QueryMsg::SwapRoutes {} => Ok(to_binary(&get_swap_routes(deps)?)?),
        QueryMsg::Ownership {} => Ok(to_binary(&cw_ownable::get_ownership(deps.storage)?)?),
        QueryMsg::Pair { pair_identifier } => {
            Ok(to_binary(&PAIRS.load(deps.storage, pair_identifier)?)?)
        }
    }
}

#[cfg(test)]
mod tests {}

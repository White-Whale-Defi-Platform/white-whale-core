#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
use white_whale::pool_network::pair::FeatureToggle;
// use cw2::set_contract_version;

use crate::{commands, queries};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, MANAGER_CONFIG};
/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:plankton-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
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
        ExecuteMsg::AddNativeTokenDecimals { denom, decimals } => {
            commands::add_native_token_decimals(deps, env, denom, decimals)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    match _msg {
        QueryMsg::NativeTokenDecimals { denom } => {
            to_binary(&queries::query_native_token_decimal(_deps, denom)?)
        }
    }
}

#[cfg(test)]
mod tests {}

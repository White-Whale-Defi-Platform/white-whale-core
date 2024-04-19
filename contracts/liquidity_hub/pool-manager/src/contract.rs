use crate::error::ContractError;
use crate::queries::{get_swap_route, get_swap_routes};
use crate::router::commands::{add_swap_routes, remove_swap_routes};
use crate::state::{Config, MANAGER_CONFIG, PAIRS, PAIR_COUNTER};
use crate::{liquidity, manager, queries, router, swap};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use cw2::set_contract_version;
use semver::Version;
use white_whale_std::pool_manager::{
    ExecuteMsg, FeatureToggle, InstantiateMsg, MigrateMsg, PairInfoResponse, QueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ww-pool-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config: Config = Config {
        whale_lair_addr: deps.api.addr_validate(&msg.fee_collector_addr)?,
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
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

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
            asset_denoms,
            asset_decimals,
            pool_fees,
            pair_type,
            pair_identifier,
        } => manager::commands::create_pair(
            deps,
            env,
            info,
            asset_denoms,
            asset_decimals,
            pool_fees,
            pair_type,
            pair_identifier,
        ),
        ExecuteMsg::ProvideLiquidity {
            slippage_tolerance,
            receiver,
            pair_identifier,
        } => liquidity::commands::provide_liquidity(
            deps,
            env,
            info,
            slippage_tolerance,
            receiver,
            pair_identifier,
        ),
        ExecuteMsg::Swap {
            offer_asset,
            ask_asset_denom,
            belief_price,
            max_spread,
            to,
            pair_identifier,
        } => {
            let to_addr = to.map(|addr| deps.api.addr_validate(&addr)).transpose()?;

            swap::commands::swap(
                deps,
                env,
                info.clone(),
                info.sender,
                offer_asset,
                ask_asset_denom,
                belief_price,
                max_spread,
                to_addr,
                pair_identifier,
            )
        }
        ExecuteMsg::WithdrawLiquidity { pair_identifier } => {
            liquidity::commands::withdraw_liquidity(deps, env, info, pair_identifier)
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
        ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
            max_spread,
        } => {
            let api = deps.api;
            router::commands::execute_swap_operations(
                deps,
                info,
                operations,
                minimum_receive,
                optional_addr_validate(api, to)?,
                max_spread,
            )
        }
        // ExecuteMsg::ExecuteSwapOperation {
        //     operation,
        //     to,
        //     max_spread,
        // } => {
        //     let api = deps.api;
        //     router::commands::execute_swap_operation(
        //         deps,
        //         env,
        //         info,
        //         operation,
        //         optional_addr_validate(api, to)?.map(|v| v.to_string()),
        //         max_spread,
        //     )
        // }
        ExecuteMsg::AddSwapRoutes { swap_routes } => {
            add_swap_routes(deps, env, info.sender, swap_routes)
        }
        ExecuteMsg::RemoveSwapRoutes { swap_routes } => {
            remove_swap_routes(deps, env, info.sender, swap_routes)
        }
        ExecuteMsg::UpdateConfig {
            whale_lair_addr,
            pool_creation_fee,
            feature_toggle,
        } => manager::update_config(
            deps,
            info,
            whale_lair_addr,
            pool_creation_fee,
            feature_toggle,
        ),
    }
}

//todo remove. solution: just embed the content of the function where it's used
// Came from router can probably go
#[allow(dead_code)]
fn optional_addr_validate(
    api: &dyn Api,
    addr: Option<String>,
) -> Result<Option<Addr>, ContractError> {
    let addr = if let Some(addr) = addr {
        Some(api.addr_validate(&addr)?)
    } else {
        None
    };

    Ok(addr)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_json_binary(&queries::query_config(deps)?)?),
        QueryMsg::AssetDecimals {
            pair_identifier,
            denom,
        } => Ok(to_json_binary(&queries::query_asset_decimals(
            deps,
            pair_identifier,
            denom,
        )?)?),
        QueryMsg::Simulation {
            offer_asset,
            pair_identifier,
        } => Ok(to_json_binary(&queries::query_simulation(
            deps,
            offer_asset,
            pair_identifier,
        )?)?),
        QueryMsg::ReverseSimulation {
            ask_asset,
            offer_asset,
            pair_identifier,
        } => Ok(to_json_binary(&queries::query_reverse_simulation(
            deps,
            env,
            ask_asset,
            offer_asset,
            pair_identifier,
        )?)?),
        // QueryMsg::SimulateSwapOperations {
        //     offer_amount,
        //     operations,
        // } => Ok(to_binary(&queries::simulate_swap_operations(
        //     deps,
        //     env,
        //     offer_amount,
        //     operations,
        // )?)?),
        // QueryMsg::ReverseSimulateSwapOperations {
        //     ask_amount,
        //     operations,
        // } => Ok(to_binary(&queries::reverse_simulate_swap_operations(
        //     deps, env, ask_amount, operations,
        // )?)?),
        QueryMsg::SwapRoute {
            offer_asset_denom,
            ask_asset_denom,
        } => Ok(to_json_binary(&get_swap_route(
            deps,
            offer_asset_denom,
            ask_asset_denom,
        )?)?),
        QueryMsg::SwapRoutes {} => Ok(to_json_binary(&get_swap_routes(deps)?)?),
        QueryMsg::Ownership {} => Ok(to_json_binary(&cw_ownable::get_ownership(deps.storage)?)?),
        QueryMsg::Pair { pair_identifier } => Ok(to_json_binary(&PairInfoResponse {
            pair_info: PAIRS.load(deps.storage, &pair_identifier)?,
        })?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use cw2::get_contract_version;
    use white_whale_std::migrate_guards::check_contract_name;

    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(ContractError::MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg(test)]
mod tests {}

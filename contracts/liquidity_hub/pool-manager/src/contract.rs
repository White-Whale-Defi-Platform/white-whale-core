use crate::error::ContractError;
use crate::helpers::{
    reverse_simulate_swap_operations, simulate_swap_operations, validate_asset_balance,
};
use crate::queries::{get_pair, get_swap_route, get_swap_route_creator, get_swap_routes};
use crate::router::commands::{add_swap_routes, remove_swap_routes};
use crate::state::{
    Config, SingleSideLiquidityProvisionBuffer, CONFIG, PAIR_COUNTER,
    SINGLE_SIDE_LIQUIDITY_PROVISION_BUFFER,
};
use crate::{liquidity, manager, queries, router, swap};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use cosmwasm_std::{wasm_execute, Reply, StdError};
use cw2::set_contract_version;
use semver::Version;
use white_whale_std::pool_manager::{
    ExecuteMsg, FeatureToggle, InstantiateMsg, MigrateMsg, QueryMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:ww-pool-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SINGLE_SIDE_LIQUIDITY_PROVISION_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config: Config = Config {
        bonding_manager_addr: deps.api.addr_validate(&msg.bonding_manager_addr)?,
        incentive_manager_addr: deps.api.addr_validate(&msg.incentive_manager_addr)?,
        // We must set a creation fee on instantiation to prevent spamming of pools
        pool_creation_fee: msg.pool_creation_fee,
        feature_toggle: FeatureToggle {
            withdrawals_enabled: true,
            deposits_enabled: true,
            swaps_enabled: true,
        },
    };
    CONFIG.save(deps.storage, &config)?;
    // initialize vault counter
    PAIR_COUNTER.save(deps.storage, &0u64)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        SINGLE_SIDE_LIQUIDITY_PROVISION_REPLY_ID => {
            let SingleSideLiquidityProvisionBuffer {
                receiver,
                expected_offer_asset_balance_in_contract,
                expected_ask_asset_balance_in_contract,
                offer_asset_half,
                expected_ask_asset,
                liquidity_provision_data,
            } = SINGLE_SIDE_LIQUIDITY_PROVISION_BUFFER.load(deps.storage)?;

            validate_asset_balance(&deps, &env, &expected_offer_asset_balance_in_contract)?;
            validate_asset_balance(&deps, &env, &expected_ask_asset_balance_in_contract)?;

            SINGLE_SIDE_LIQUIDITY_PROVISION_BUFFER.remove(deps.storage);

            Ok(Response::default().add_message(wasm_execute(
                env.contract.address.into_string(),
                &ExecuteMsg::ProvideLiquidity {
                    slippage_tolerance: liquidity_provision_data.slippage_tolerance,
                    max_spread: liquidity_provision_data.max_spread,
                    receiver: Some(receiver),
                    pair_identifier: liquidity_provision_data.pair_identifier,
                    unlocking_duration: liquidity_provision_data.unlocking_duration,
                    lock_position_identifier: liquidity_provision_data.lock_position_identifier,
                },
                vec![offer_asset_half, expected_ask_asset],
            )?))
        }
        _ => Err(StdError::generic_err("reply id not found").into()),
    }
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
            max_spread,
            slippage_tolerance,
            receiver,
            pair_identifier,
            unlocking_duration,
            lock_position_identifier,
        } => liquidity::commands::provide_liquidity(
            deps,
            env,
            info,
            slippage_tolerance,
            max_spread,
            receiver,
            pair_identifier,
            unlocking_duration,
            lock_position_identifier,
        ),
        ExecuteMsg::Swap {
            ask_asset_denom,
            belief_price,
            max_spread,
            receiver,
            pair_identifier,
        } => swap::commands::swap(
            deps,
            env,
            info.clone(),
            info.sender,
            ask_asset_denom,
            belief_price,
            max_spread,
            receiver,
            pair_identifier,
        ),
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
        ExecuteMsg::AddSwapRoutes { swap_routes } => {
            add_swap_routes(deps, info.sender, swap_routes)
        }
        ExecuteMsg::RemoveSwapRoutes { swap_routes } => {
            remove_swap_routes(deps, info.sender, swap_routes)
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
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations,
        } => Ok(to_json_binary(&simulate_swap_operations(
            deps,
            offer_amount,
            operations,
        )?)?),
        QueryMsg::ReverseSimulateSwapOperations {
            ask_amount,
            operations,
        } => Ok(to_json_binary(&reverse_simulate_swap_operations(
            deps, ask_amount, operations,
        )?)?),
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
        QueryMsg::Pair { pair_identifier } => {
            Ok(to_json_binary(&get_pair(deps, pair_identifier)?)?)
        }
        QueryMsg::SwapRouteCreator {
            offer_asset_denom,
            ask_asset_denom,
        } => Ok(to_json_binary(&get_swap_route_creator(
            deps,
            offer_asset_denom,
            ask_asset_denom,
        )?)?),
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

use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Api, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;
use semver::Version;

use pool_network::asset::{Asset, AssetInfo, PairInfo};
use pool_network::pair::SimulationResponse;
use pool_network::querier::{query_pair_info, reverse_simulate, simulate};
use pool_network::router::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
    SimulateSwapOperationsResponse, SwapOperation, SwapRoute,
};

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::operations::execute_swap_operation;
use crate::state::{Config, CONFIG, SWAP_ROUTES};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-pool_router";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    CONFIG.save(
        deps.storage,
        &Config {
            terraswap_factory: deps.api.addr_canonicalize(&msg.terraswap_factory)?,
        },
    )?;

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
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
        } => {
            let api = deps.api;
            execute_swap_operations(
                deps,
                env,
                info.sender,
                operations,
                minimum_receive,
                optional_addr_validate(api, to)?,
            )
        }
        ExecuteMsg::ExecuteSwapOperation { operation, to } => {
            let api = deps.api;
            execute_swap_operation(
                deps,
                env,
                info,
                operation,
                optional_addr_validate(api, to)?.map(|v| v.to_string()),
            )
        }
        ExecuteMsg::AssertMinimumReceive {
            asset_info,
            prev_balance,
            minimum_receive,
            receiver,
        } => assert_minimum_receive(
            deps.as_ref(),
            asset_info,
            prev_balance,
            minimum_receive,
            deps.api.addr_validate(&receiver)?,
        ),
        ExecuteMsg::AddSwapRoutes { swap_routes } => {
            add_swap_routes(deps, env, info.sender, swap_routes)
        }
    }
}

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

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender = deps.api.addr_validate(&cw20_msg.sender)?;
    match from_binary(&cw20_msg.msg)? {
        Cw20HookMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
        } => {
            let api = deps.api;
            execute_swap_operations(
                deps,
                env,
                sender,
                operations,
                minimum_receive,
                optional_addr_validate(api, to)?,
            )
        }
    }
}

pub fn execute_swap_operations(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    operations: Vec<SwapOperation>,
    minimum_receive: Option<Uint128>,
    to: Option<Addr>,
) -> Result<Response, ContractError> {
    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(StdError::generic_err("Must provide swap operations to execute").into());
    }

    // Assert the operations are properly set
    assert_operations(&operations)?;

    let to = if let Some(to) = to { to } else { sender };
    let target_asset_info = operations
        .last()
        .ok_or_else(|| ContractError::Std(StdError::generic_err("Couldn't get swap operation")))?
        .get_target_asset_info();

    let mut operation_index = 0;
    let mut messages: Vec<CosmosMsg> = operations
        .into_iter()
        .map(|op| {
            operation_index += 1;
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                funds: vec![],
                msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
                    operation: op,
                    to: if operation_index == operations_len {
                        Some(to.to_string())
                    } else {
                        None
                    },
                })?,
            }))
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    // Execute minimum amount assertion
    if let Some(minimum_receive) = minimum_receive {
        let receiver_balance = target_asset_info.query_pool(&deps.querier, deps.api, to.clone())?;

        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            funds: vec![],
            msg: to_binary(&ExecuteMsg::AssertMinimumReceive {
                asset_info: target_asset_info,
                prev_balance: receiver_balance,
                minimum_receive,
                receiver: to.to_string(),
            })?,
        }))
    }

    Ok(Response::new().add_messages(messages))
}

fn assert_minimum_receive(
    deps: Deps,
    asset_info: AssetInfo,
    prev_balance: Uint128,
    minimum_receive: Uint128,
    receiver: Addr,
) -> Result<Response, ContractError> {
    let receiver_balance = asset_info.query_pool(&deps.querier, deps.api, receiver)?;
    let swap_amount = receiver_balance.checked_sub(prev_balance)?;

    if swap_amount < minimum_receive {
        return Err(ContractError::MinimumReceiveAssertion {
            minimum_receive,
            swap_amount,
        });
    }

    Ok(Response::default())
}

fn add_swap_routes(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    swap_routes: Vec<SwapRoute>,
) -> Result<Response, ContractError> {
    let contract_info = deps
        .querier
        .query_wasm_contract_info(env.contract.address)?;
    if let Some(admin) = contract_info.admin {
        if sender != deps.api.addr_validate(admin.as_str())? {
            return Err(ContractError::Unauthorized {});
        }
    }

    let mut attributes = vec![];

    for swap_route in swap_routes {
        simulate_swap_operations(
            deps.as_ref(),
            Uint128::one(),
            swap_route.clone().swap_operations,
        )
        .map_err(|_| ContractError::InvalidSwapRoute(swap_route.clone()))?;

        let swap_route_key = SWAP_ROUTES.key((
            swap_route
                .clone()
                .offer_asset_info
                .get_label(&deps.as_ref())?
                .as_str(),
            swap_route
                .clone()
                .ask_asset_info
                .get_label(&deps.as_ref())?
                .as_str(),
        ));
        swap_route_key.save(deps.storage, &swap_route.clone().swap_operations)?;

        attributes.push(attr("swap_route", swap_route.clone().to_string()));
    }

    Ok(Response::new()
        .add_attribute("action", "add_swap_routes")
        .add_attributes(attributes))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&query_config(deps)?)?),
        QueryMsg::SimulateSwapOperations {
            offer_amount,
            operations,
        } => Ok(to_binary(&simulate_swap_operations(
            deps,
            offer_amount,
            operations,
        )?)?),
        QueryMsg::ReverseSimulateSwapOperations {
            ask_amount,
            operations,
        } => Ok(to_binary(&reverse_simulate_swap_operations(
            deps, ask_amount, operations,
        )?)?),
        QueryMsg::SwapRoute {
            offer_asset_info,
            ask_asset_info,
        } => Ok(to_binary(&get_swap_route(
            deps,
            offer_asset_info,
            ask_asset_info,
        )?)?),
    }
}

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let state = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        terraswap_factory: deps
            .api
            .addr_humanize(&state.terraswap_factory)?
            .to_string(),
    };

    Ok(resp)
}

fn simulate_swap_operations(
    deps: Deps,
    offer_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationsResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let terraswap_factory = deps.api.addr_humanize(&config.terraswap_factory)?;

    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(ContractError::NoSwapOperationsProvided {});
    }

    let mut offer_amount = offer_amount;
    for operation in operations.into_iter() {
        match operation {
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
            } => {
                let pair_info: PairInfo = query_pair_info(
                    &deps.querier,
                    terraswap_factory.clone(),
                    &[offer_asset_info.clone(), ask_asset_info.clone()],
                )?;

                let res: SimulationResponse = simulate(
                    &deps.querier,
                    Addr::unchecked(pair_info.contract_addr),
                    &Asset {
                        info: offer_asset_info,
                        amount: offer_amount,
                    },
                )?;

                offer_amount = res.return_amount;
            }
        }
    }

    Ok(SimulateSwapOperationsResponse {
        amount: offer_amount,
    })
}

fn reverse_simulate_swap_operations(
    deps: Deps,
    ask_amount: Uint128,
    operations: Vec<SwapOperation>,
) -> Result<SimulateSwapOperationsResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    let operations_len = operations.len();
    if operations_len == 0 {
        return Err(ContractError::NoSwapOperationsProvided {});
    }

    let mut ask_amount = ask_amount;
    for operation in operations.into_iter().rev() {
        ask_amount = match operation {
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
            } => {
                let terraswap_factory = deps.api.addr_humanize(&config.terraswap_factory)?;

                reverse_simulate_return_amount(
                    deps,
                    terraswap_factory,
                    ask_amount,
                    offer_asset_info,
                    ask_asset_info,
                )?
            }
        }
    }

    Ok(SimulateSwapOperationsResponse { amount: ask_amount })
}

fn reverse_simulate_return_amount(
    deps: Deps,
    factory: Addr,
    ask_amount: Uint128,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> StdResult<Uint128> {
    let pair_info: PairInfo = query_pair_info(
        &deps.querier,
        factory,
        &[offer_asset_info, ask_asset_info.clone()],
    )?;

    let res = reverse_simulate(
        &deps.querier,
        Addr::unchecked(pair_info.contract_addr),
        &Asset {
            amount: ask_amount,
            info: ask_asset_info,
        },
    )?;

    Ok(res.offer_amount)
}

fn get_swap_route(
    deps: Deps,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<Vec<SwapOperation>, ContractError> {
    let swap_route_key = SWAP_ROUTES.key((
        offer_asset_info.clone().get_label(&deps)?.as_str(),
        ask_asset_info.clone().get_label(&deps)?.as_str(),
    ));

    swap_route_key
        .load(deps.storage)
        .map_err(|_| ContractError::NoSwapRouteForAssets {
            offer_asset: offer_asset_info.to_string(),
            ask_asset: ask_asset_info.to_string(),
        })
}

fn assert_operations(operations: &[SwapOperation]) -> Result<(), ContractError> {
    let mut ask_asset_map: HashMap<String, bool> = HashMap::new();
    for operation in operations.iter() {
        let (offer_asset, ask_asset) = match operation {
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
            } => (offer_asset_info.clone(), ask_asset_info.clone()),
        };

        ask_asset_map.remove(&offer_asset.to_string());
        ask_asset_map.insert(ask_asset.to_string(), true);
    }

    if ask_asset_map.keys().len() != 1 {
        return Err(ContractError::MultipleOutputToken {});
    }

    Ok(())
}

#[test]
fn test_invalid_operations() {
    // empty error
    assert!(assert_operations(&[]).is_err());

    // uluna output
    assert!(assert_operations(&[
        SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        },
        SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        }
    ])
    .is_ok());

    // asset0002 output
    assert!(assert_operations(&[
        SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "ukrw".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
        },
        SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::Token {
                contract_addr: "asset0001".to_string(),
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        },
        SwapOperation::TerraSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            ask_asset_info: AssetInfo::Token {
                contract_addr: "asset0002".to_string(),
            },
        },
    ])
    .is_ok());
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

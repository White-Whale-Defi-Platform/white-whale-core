use cosmwasm_std::{
    to_binary, Addr, BalanceResponse, BankQuery, Coin, CosmosMsg, DepsMut, Env, MessageInfo,
    QueryRequest, Response, StdResult, Storage, Uint128, WasmMsg, WasmQuery,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg};

use pool_network::asset::AssetInfo;
use pool_network::factory::{PairsResponse, QueryMsg};
use pool_network::router;
use pool_network::router::{ExecuteMsg, SwapOperation};
use vault_network::vault_factory::VaultsResponse;

use crate::msg::{ContractType, FactoryType, FeesFor};
use crate::state::{read_temporal_asset_infos, store_temporal_asset_info, Config, CONFIG};
use crate::ContractError;

/// Collects fees accrued by the pools and vaults. If a factory is provided then it only collects the
/// fees from its children.
pub fn collect_fees(
    deps: DepsMut,
    info: MessageInfo,
    collect_fees_for: FeesFor,
) -> Result<Response, ContractError> {
    // only the owner can trigger the fees collection
    validate_owner(deps.storage, info.sender)?;

    let mut collect_fees_messages: Vec<CosmosMsg> = Vec::new();

    match collect_fees_for {
        FeesFor::Contracts { contracts } => {
            for contract in contracts {
                collect_fees_messages.push(collect_fees_for_contract(
                    deps.api.addr_validate(contract.address.as_str())?,
                    contract.contract_type,
                )?);
            }
        }
        FeesFor::Factory {
            factory_addr,
            factory_type,
        } => {
            let factory = deps.api.addr_validate(factory_addr.as_str())?;
            collect_fees_messages = collect_fees_for_factory(&deps, &factory, factory_type)?;
        }
    }

    Ok(Response::new()
        .add_attribute("action", "collect_fees")
        .add_messages(collect_fees_messages))
}

/// Builds the message to collect the fees for the given contract
fn collect_fees_for_contract(contract: Addr, contract_type: ContractType) -> StdResult<CosmosMsg> {
    let collect_protocol_fees_msg = match contract_type {
        ContractType::Vault {} => {
            to_binary(&vault_network::vault::ExecuteMsg::CollectProtocolFees {})?
        }
        ContractType::Pool {} => {
            to_binary(&pool_network::pair::ExecuteMsg::CollectProtocolFees {})?
        }
    };

    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: collect_protocol_fees_msg,
        funds: vec![],
    }))
}

/// Builds the messages to collect the fees for the given factory's children.
fn collect_fees_for_factory(
    deps: &DepsMut,
    factory: &Addr,
    factory_type: FactoryType,
) -> StdResult<Vec<CosmosMsg>> {
    let mut result: Vec<CosmosMsg> = Vec::new();

    match factory_type {
        FactoryType::Vault { start_after, limit } => {
            let response: VaultsResponse =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: factory.to_string(),
                    msg: to_binary(&vault_network::vault_factory::QueryMsg::Vaults {
                        start_after,
                        limit,
                    })?,
                }))?;

            for vault_info in response.vaults {
                result.push(collect_fees_for_contract(
                    deps.api.addr_validate(vault_info.vault.as_str())?,
                    ContractType::Vault {},
                )?);
            }
        }
        FactoryType::Pool { start_after, limit } => {
            let response: PairsResponse =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: factory.to_string(),
                    msg: to_binary(&QueryMsg::Pairs { start_after, limit })?,
                }))?;

            for pair in response.pairs {
                result.push(collect_fees_for_contract(
                    deps.api
                        .addr_validate(pair.clone().contract_addr.as_str())?,
                    ContractType::Pool {},
                )?);
            }
        }
    }

    Ok(result)
}

/// Validates that the given sender [Addr] is the owner of the contract
fn validate_owner(storage: &dyn Storage, sender: Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(storage)?;
    if sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    pool_router: Option<String>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_validate(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        let owner_addr = deps.api.addr_validate(&owner)?;
        config.owner = owner_addr;
    }

    if let Some(pool_router) = pool_router {
        let pool_router = deps.api.addr_validate(&pool_router)?;
        config.pool_router = pool_router;
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Aggregates the fees collected into the given asset_info.
pub fn aggregate_fees(
    mut deps: DepsMut,
    info: MessageInfo,
    env: Env,
    ask_asset_info: AssetInfo,
    aggregate_fees_for: FeesFor,
) -> Result<Response, ContractError> {
    // only the owner can aggregate the fees
    validate_owner(deps.storage, info.sender)?;
    let config: Config = CONFIG.load(deps.storage)?;

    let mut aggregate_fees_messages: Vec<CosmosMsg> = Vec::new();

    match aggregate_fees_for {
        FeesFor::Contracts { .. } => return Err(ContractError::InvalidContractsFeeAggregation {}),
        FeesFor::Factory {
            factory_addr,
            factory_type,
        } => {
            let factory = deps.api.addr_validate(factory_addr.as_str())?;

            match factory_type {
                FactoryType::Vault { start_after, limit } => {
                    let response: VaultsResponse =
                        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                            contract_addr: factory.to_string(),
                            msg: to_binary(&vault_network::vault_factory::QueryMsg::Vaults {
                                start_after,
                                limit,
                            })?,
                        }))?;

                    for vault_info in response.vaults {
                        store_temporal_asset_info(deps.branch(), vault_info.asset_info.clone())?;
                    }
                }
                FactoryType::Pool { start_after, limit } => {
                    let response: PairsResponse =
                        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                            contract_addr: factory.to_string(),
                            msg: to_binary(&QueryMsg::Pairs { start_after, limit })?,
                        }))?;

                    for pair in response.pairs {
                        store_temporal_asset_info(deps.branch(), pair.asset_infos[0].clone())?;
                        store_temporal_asset_info(deps.branch(), pair.asset_infos[1].clone())?;
                    }
                }
            }
        }
    }

    let asset_infos: Vec<AssetInfo> = read_temporal_asset_infos(&mut deps)?;

    for offer_asset_info in asset_infos {
        if offer_asset_info == ask_asset_info {
            continue;
        }

        // get balance of the asset to aggregate
        let balance: Uint128 = match offer_asset_info.clone() {
            AssetInfo::Token { contract_addr } => {
                let contract_addr = deps.api.addr_validate(contract_addr.as_str())?;
                let balance_response: cw20::BalanceResponse =
                    deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                        contract_addr: contract_addr.to_string(),
                        msg: to_binary(&Cw20QueryMsg::Balance {
                            address: env.contract.address.to_string(),
                        })?,
                    }))?;

                if balance_response.balance > Uint128::zero() {
                    // if balance is greater than zero, some swap will occur
                    // Increase the allowance for the cw20 token so the router can perform the swap
                    aggregate_fees_messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract_addr.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::IncreaseAllowance {
                            spender: config.pool_router.to_string(),
                            amount: balance_response.balance,
                            expires: None,
                        })?,
                        funds: vec![],
                    }));
                }

                balance_response.balance
            }
            AssetInfo::NativeToken { denom } => {
                let balance_response: BalanceResponse =
                    deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
                        address: env.contract.address.to_string(),
                        denom: denom.clone(),
                    }))?;
                balance_response.amount.amount
            }
        };

        // if the balance is greater than zero, swap the asset to the ask_asset
        if balance > Uint128::zero() {
            // query swap route from router
            let operations_res: StdResult<Vec<SwapOperation>> =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: config.pool_router.to_string(),
                    msg: to_binary(&router::QueryMsg::SwapRoute {
                        offer_asset_info: offer_asset_info.clone(),
                        ask_asset_info: ask_asset_info.clone(),
                    })?,
                }));

            match operations_res {
                Ok(operations) => {
                    let execute_swap_operations_msg =
                        to_binary(&ExecuteMsg::ExecuteSwapOperations {
                            operations,
                            minimum_receive: None,
                            to: None,
                        })?;

                    match offer_asset_info.clone() {
                        AssetInfo::Token { contract_addr } => {
                            aggregate_fees_messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr,
                                funds: vec![],
                                msg: to_binary(&Cw20ExecuteMsg::Send {
                                    contract: config.pool_router.to_string(),
                                    amount: balance,
                                    msg: execute_swap_operations_msg,
                                })?,
                            }));
                        }
                        AssetInfo::NativeToken { denom } => {
                            aggregate_fees_messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: config.pool_router.to_string(),
                                funds: vec![Coin {
                                    denom,
                                    amount: balance,
                                }],
                                msg: execute_swap_operations_msg,
                            }));
                        }
                    };
                }
                Err(_) => {
                    // if there is no swap route, skip swap and keep the asset in contract
                    continue;
                }
            };
        }
    }

    Ok(Response::new()
        .add_attribute("action", "aggregate_fees")
        .add_messages(aggregate_fees_messages))
}

use cosmwasm_std::{to_binary, Addr, Deps, QueryRequest, StdResult, WasmQuery};
use terraswap::asset::Asset;
use terraswap::factory::{PairsResponse, QueryMsg};
use terraswap::pair::ProtocolFeesResponse as ProtocolPairFeesResponse;
use vault_network::vault::ProtocolFeesResponse as ProtocolVaultFeesResponse;
use vault_network::vault_factory::VaultsResponse;

use crate::msg::{ContractType, FactoriesResponse, FactoryType, QueryFeesFor};
use crate::state::{read_factories, ConfigResponse, CONFIG};

pub fn query_factories(deps: Deps, limit: Option<u32>) -> StdResult<FactoriesResponse> {
    let factories = read_factories(deps, limit)?;
    Ok(FactoriesResponse { factories })
}
/// Queries the [Config], which contains the owner address
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

pub fn query_fees(
    deps: Deps,
    collect_fees_for: QueryFeesFor,
    accrued: bool,
) -> StdResult<Vec<Asset>> {
    let mut query_fees_messages: Vec<Asset> = Vec::new();

    match collect_fees_for {
        QueryFeesFor::Contracts { contracts } => {
            for contract in contracts {
                match contract.contract_type {
                    ContractType::Pool {} => {
                        let mut pair_fee =
                            query_fees_for_pair(&deps, contract.address.clone(), accrued)?;

                        query_fees_messages.append(&mut pair_fee);
                    }
                    ContractType::Vault {} => {
                        let vault_fee =
                            query_fees_for_vault(&deps, contract.address.clone(), accrued)?;

                        query_fees_messages.push(vault_fee);
                    }
                }
            }
        }
        QueryFeesFor::Factory {
            factory_addr,
            factory_type,
        } => {
            let factory = deps.api.addr_validate(factory_addr.as_str())?;
            let mut assets = query_fees_for_factory(&deps, &factory, factory_type, accrued)?;

            query_fees_messages.append(&mut assets);
        }
    }

    Ok(query_fees_messages)
}

fn query_fees_for_vault(deps: &Deps, vault: String, accrued: bool) -> StdResult<Asset> {
    let all_time = if accrued {
        None
    } else {
        let fees = deps
            .querier
            .query::<ProtocolVaultFeesResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: vault.clone(),
                msg: to_binary(&vault_network::vault::QueryMsg::ProtocolFees { all_time: true })?,
            }))?
            .fees;

        Some(fees)
    };

    let mut asset = deps
        .querier
        .query::<ProtocolVaultFeesResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: vault,
            msg: to_binary(&vault_network::vault::QueryMsg::ProtocolFees { all_time: false })?,
        }))?
        .fees;

    if let Some(all_time_fees) = all_time {
        asset.amount = all_time_fees.amount.checked_sub(asset.amount)?;
    }

    Ok(asset)
}

fn query_fees_for_pair(deps: &Deps, pair: String, accrued: bool) -> StdResult<Vec<Asset>> {
    let all_time = if accrued {
        vec![]
    } else {
        deps.querier
            .query::<ProtocolPairFeesResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: pair.clone(),
                msg: to_binary(&terraswap::pair::QueryMsg::ProtocolFees {
                    all_time: Some(true),
                    asset_id: None,
                })?,
            }))?
            .fees
    };

    let mut accrued = deps
        .querier
        .query::<ProtocolPairFeesResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: pair,
            msg: to_binary(&terraswap::pair::QueryMsg::ProtocolFees {
                all_time: None,
                asset_id: None,
            })?,
        }))?
        .fees;

    for mut asset in &mut accrued {
        let all_time_result = all_time
            .iter()
            .find(|asset_all_time| asset_all_time.info == asset.info);

        if let Some(all_time_asset) = all_time_result {
            asset.amount = all_time_asset.amount.checked_sub(asset.amount)?;
        }
    }

    Ok(accrued)
}

fn query_fees_for_factory(
    deps: &Deps,
    factory: &Addr,
    factory_type: FactoryType,
    accrued: bool,
) -> StdResult<Vec<Asset>> {
    let mut query_fees_messages: Vec<Asset> = Vec::new();

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
                let vault_fee = query_fees_for_vault(deps, vault_info.vault, accrued)?;
                query_fees_messages.push(vault_fee);
            }
        }
        FactoryType::Pool { start_after, limit } => {
            let response: PairsResponse =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: factory.to_string(),
                    msg: to_binary(&QueryMsg::Pairs { start_after, limit })?,
                }))?;

            for pair in response.pairs {
                let mut pair_fees = query_fees_for_pair(deps, pair.contract_addr, accrued)?;
                query_fees_messages.append(&mut pair_fees);
            }
        }
    }

    Ok(query_fees_messages)
}

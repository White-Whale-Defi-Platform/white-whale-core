use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, MessageInfo, QueryRequest, Response, StdResult, Storage,
    WasmMsg, WasmQuery,
};

use terraswap::factory::{PairsResponse, QueryMsg};
use vault_network::vault_factory::VaultsResponse;

use crate::msg::{CollectFeesFor, ContractType, FactoryType};
use crate::state::{Config, CONFIG, FACTORIES};
use crate::ContractError;

/// Adds a factory to the list of factories so it can be queried when collecting fees
pub fn add_factory(
    deps: DepsMut,
    info: MessageInfo,
    factory_addr: String,
) -> Result<Response, ContractError> {
    validate_owner(deps.storage, info.sender)?;

    let factory = deps.api.addr_validate(factory_addr.as_str())?;
    let factory_key = factory.as_bytes();
    FACTORIES.save(deps.storage, factory_key, &factory)?;

    Ok(Response::new()
        .add_attribute("action", "add_factory")
        .add_attribute("factory", factory.as_str()))
}

/// Removes a factory to the list of factories
pub fn remove_factory(
    deps: DepsMut,
    info: MessageInfo,
    factory_addr: String,
) -> Result<Response, ContractError> {
    validate_owner(deps.storage, info.sender)?;

    let factory = deps.api.addr_validate(factory_addr.as_str())?;
    let factory_key = factory.as_bytes();

    FACTORIES.remove(deps.storage, factory_key);

    Ok(Response::new()
        .add_attribute("action", "remove_factory")
        .add_attribute("factory", factory.as_str()))
}

/// Collects fees accrued by the pools and vaults. If a factory is provided then it only collects the
/// fees from its children.
pub fn collect_fees(
    deps: DepsMut,
    info: MessageInfo,
    collect_fees_for: CollectFeesFor,
) -> Result<Response, ContractError> {
    // only the owner can trigger the fees collection
    validate_owner(deps.storage, info.sender)?;

    let mut collect_fees_messages: Vec<CosmosMsg> = Vec::new();

    match collect_fees_for {
        CollectFeesFor::Contracts { contracts } => {
            for contract in contracts {
                collect_fees_messages.push(collect_fees_for_contract(
                    deps.api.addr_validate(contract.address.as_str())?,
                    contract.contract_type,
                )?);
            }
        }
        CollectFeesFor::Factory {
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
        ContractType::Pool {} => to_binary(&terraswap::pair::ExecuteMsg::CollectProtocolFees {})?,
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
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_validate(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        // validate address format
        let owner_addr = deps.api.addr_validate(&owner)?;
        config.owner = owner_addr;
    }

    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_config"))
}

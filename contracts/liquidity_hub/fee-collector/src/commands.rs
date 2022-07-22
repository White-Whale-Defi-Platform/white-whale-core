use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, MessageInfo, QueryRequest, Response, StdResult, Storage,
    WasmMsg, WasmQuery,
};

use terraswap::asset::AssetInfo;
use terraswap::factory::{PairsResponse, QueryMsg};
use terraswap::pair::ExecuteMsg::CollectProtocolFees;

use crate::state::{read_factories, Config, CONFIG, FACTORIES};
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
    factory_addr: Option<String>,
    contracts: Option<Vec<String>>,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    // only the owner can trigger the fees collection
    validate_owner(deps.storage, info.sender)?;

    let mut collect_fees_messages: Vec<CosmosMsg> = Vec::new();

    if let Some(contracts) = contracts {
        for contract in contracts {
            collect_fees_messages.push(collect_fees_for_contract(
                deps.api.addr_validate(contract.as_str())?,
            )?);
        }
    } else if let Some(factory_addr) = factory_addr {
        let factory = deps.api.addr_validate(factory_addr.as_str())?;
        collect_fees_messages = collect_fees_for_factory(&deps, &factory, start_after, limit)?;
    } else {
        let factories = read_factories(deps.as_ref(), None)?;

        for factory in factories {
            collect_fees_messages.append(&mut collect_fees_for_factory(
                &deps,
                &factory,
                start_after.clone(),
                limit,
            )?);
        }
    }

    Ok(Response::new()
        .add_attribute("action", "collect_fees")
        .add_messages(collect_fees_messages))
}

/// Builds the message to collect the fees for the given contract
fn collect_fees_for_contract(contract: Addr) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract.to_string(),
        msg: to_binary(&CollectProtocolFees {})?,
        funds: vec![],
    }))
}

/// Builds the messages to collect the fees for the given factory's children.
fn collect_fees_for_factory(
    deps: &DepsMut,
    factory: &Addr,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<Vec<CosmosMsg>> {
    let response: PairsResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: factory.to_string(),
        msg: to_binary(&QueryMsg::Pairs { start_after, limit })?,
    }))?;

    let mut result: Vec<CosmosMsg> = Vec::new();

    for pair in response.pairs {
        result.push(collect_fees_for_contract(
            deps.api
                .addr_validate(pair.clone().contract_addr.as_str())?,
        )?);
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

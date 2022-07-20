use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, MessageInfo, QueryRequest, Response, StdResult, Storage,
    WasmMsg, WasmQuery,
};
use terraswap::asset::AssetInfo;

use crate::ContractError;
use terraswap::factory::{PairsResponse, QueryMsg};
use terraswap::pair::ExecuteMsg::CollectProtocolFees;

use crate::state::{read_factories, CONFIG, FACTORIES};

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
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    // only the owner can trigger the fees collection
    validate_owner(deps.storage, info.sender)?;

    let mut collect_fees_messages: Vec<CosmosMsg> = Vec::new();

    if let Some(factory_addr) = factory_addr {
        let factory = deps.api.addr_validate(factory_addr.as_str())?;
        collect_fees_messages = collect_fees_for_factory(&deps, &factory, start_after, limit)?;
    } else {
        let factories = read_factories(deps.as_ref(), None, None)?;

        for factory in factories {
            collect_fees_messages
                .append(&mut collect_fees_for_factory(&deps, &factory, None, None)?);
        }
    }

    Ok(Response::new()
        .add_attribute("action", "collect_fees")
        .add_messages(collect_fees_messages.clone()))
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

    response
        .pairs
        .iter()
        .map(|pair_info| {
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_info.contract_addr.to_string(),
                msg: to_binary(&CollectProtocolFees {})?,
                funds: vec![],
            }))
        })
        .collect()
}

/// Validates that the given sender [Addr] is the owner of the contract
fn validate_owner(storage: &dyn Storage, sender: Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(storage)?;
    if sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    Ok(())
}

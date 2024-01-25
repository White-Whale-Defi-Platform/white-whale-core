#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Reply,
    Response, StdResult, WasmQuery,
};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::parse_reply_execute_data;

use crate::error::ContractError;
use crate::helpers::{validate_epoch_config, validate_grace_period};
use crate::state::{get_expiring_epoch, CONFIG, EPOCHS};
use crate::{commands, migrations, queries, state};
use semver::Version;
use white_whale_std::fee_collector::ForwardFeesResponse;
use white_whale_std::fee_distributor::{
    Config, Epoch, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};
use white_whale_std::pool_network::asset;
use white_whale_std::whale_lair::GlobalIndex;
use white_whale_std::whale_lair::QueryMsg as LairQueryMsg;

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-fee_distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const EPOCH_CREATION_REPLY_ID: u64 = 1;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    validate_grace_period(&msg.grace_period)?;
    validate_epoch_config(&msg.epoch_config)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        bonding_contract_addr: deps.api.addr_validate(msg.bonding_contract_addr.as_str())?,
        fee_collector_addr: deps.api.addr_validate(msg.fee_collector_addr.as_str())?,
        grace_period: msg.grace_period,
        epoch_config: msg.epoch_config,
        distribution_asset: msg.distribution_asset,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", config.owner.as_str())
        .add_attribute(
            "bonding_contract_addr",
            config.bonding_contract_addr.as_str(),
        )
        .add_attribute("fee_collector_addr", config.fee_collector_addr.as_str())
        .add_attribute("grace_period", config.grace_period.to_string())
        .add_attribute("epoch_config", config.epoch_config.to_string())
        .add_attribute("distribution_asset", config.distribution_asset.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id == EPOCH_CREATION_REPLY_ID {
        // Read the epoch sent by the fee collector through the ForwardFeesResponse
        let execute_contract_response = parse_reply_execute_data(msg)?;
        let data = execute_contract_response
            .data
            .ok_or(ContractError::CannotReadEpoch {})?;
        let forward_fees_response: ForwardFeesResponse = from_json(data)?;
        let mut new_epoch = forward_fees_response.epoch;

        // Query bonding contract for GlobalIndex weight
        let config = queries::query_config(deps.as_ref())?;
        // Query the current global index
        let global_index: GlobalIndex =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: config.bonding_contract_addr.to_string(),
                msg: to_json_binary(&LairQueryMsg::GlobalIndex {})?,
            }))?;
        new_epoch.global_index = global_index;

        // forward fees from the expiring epoch to the new one.
        let mut expiring_epoch = get_expiring_epoch(deps.as_ref())?;

        if let Some(expiring_epoch) = expiring_epoch.as_mut() {
            let unclaimed_fees = expiring_epoch.available.clone();

            // aggregate the unclaimed fees from the expiring epoch with the ones of the new epoch
            let fees = asset::aggregate_assets(new_epoch.total, unclaimed_fees)?;
            new_epoch = Epoch {
                total: fees.clone(),
                available: fees,
                ..new_epoch
            };

            // update the expiring epoch's available fees
            expiring_epoch.available = vec![];
            EPOCHS.save(
                deps.storage,
                &expiring_epoch.id.to_be_bytes(),
                expiring_epoch,
            )?;
        }

        // save the new epoch
        EPOCHS.save(deps.storage, &new_epoch.id.to_be_bytes(), &new_epoch)?;

        Ok(Response::default()
            .add_attribute("action", "reply")
            .add_attribute("new_epoch", new_epoch.to_string())
            .add_attribute(
                "expiring_epoch",
                expiring_epoch.unwrap_or_default().to_string(),
            ))
    } else {
        Err(ContractError::UnknownReplyId(msg.id))
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::NewEpoch {} => commands::create_new_epoch(deps, env),
        ExecuteMsg::Claim {} => commands::claim(deps, info),
        ExecuteMsg::UpdateConfig {
            owner,
            bonding_contract_addr,
            fee_collector_addr,
            grace_period,
            distribution_asset,
            epoch_config,
        } => commands::update_config(
            deps,
            info,
            owner,
            bonding_contract_addr,
            fee_collector_addr,
            grace_period,
            distribution_asset,
            epoch_config,
        ),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CurrentEpoch {} => Ok(to_json_binary(&state::get_current_epoch(deps)?)?),
        QueryMsg::Epoch { id } => Ok(to_json_binary(&state::get_epoch(deps, id)?)?),
        QueryMsg::ClaimableEpochs {} => Ok(to_json_binary(&state::get_claimable_epochs(deps)?)?),
        QueryMsg::Config {} => Ok(to_json_binary(&queries::query_config(deps)?)?),
        QueryMsg::Claimable { address } => Ok(to_json_binary(&state::query_claimable(
            deps,
            &deps.api.addr_validate(&address)?,
        )?)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
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

    if storage_version < Version::parse("0.9.0")? {
        migrations::migrate_to_v090(deps.branch())?;
    }

    if storage_version == Version::parse("0.9.0")? {
        let fees_refund_messages = migrations::migrate_to_v091(deps.branch())?;
        return Ok(Response::default()
            .add_messages(fees_refund_messages)
            .add_attribute("action", "migrate"));
    }

    Ok(Response::default())
}

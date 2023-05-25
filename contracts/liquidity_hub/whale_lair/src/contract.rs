use cosmwasm_std::{entry_point, Reply, StdError, WasmMsg, SubMsg, CosmosMsg};
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;
use white_whale::pool_network::asset::{AssetInfo, Asset};

use white_whale::whale_lair::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use white_whale::fee_distributor::ExecuteMsg as FeeDistributorExecuteMsg;
use crate::commands::bond;
use crate::error::ContractError;
use crate::helpers::validate_growth_rate;
use crate::state::{BONDING_ASSETS_LIMIT, CONFIG, TEMP_INFO};
use crate::{commands, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-whale_lair";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.bonding_assets.len() > BONDING_ASSETS_LIMIT {
        return Err(ContractError::InvalidBondingAssetsLimit(
            BONDING_ASSETS_LIMIT,
            msg.bonding_assets.len(),
        ));
    }

    validate_growth_rate(msg.growth_rate)?;

    for asset in &msg.bonding_assets {
        match asset {
            AssetInfo::Token { .. } => return Err(ContractError::InvalidBondingAsset {}),
            AssetInfo::NativeToken { .. } => {}
        };
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        unbonding_period: msg.unbonding_period,
        growth_rate: msg.growth_rate,
        bonding_assets: msg.bonding_assets,
    };

    CONFIG.save(deps.storage, &config)?;

    let bonding_assets = config
        .bonding_assets
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", config.owner.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
        ("bonding_assets", bonding_assets),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bond { asset } => handle_claim_then_bond(deps, env, info, asset, true),
        ExecuteMsg::Unbond { asset } => handle_claim_then_bond(deps, env, info, asset, false),
        ExecuteMsg::Withdraw { denom } => {
            commands::withdraw(deps, env.block.time, info.sender, denom)
        }
        ExecuteMsg::UpdateConfig {
            owner,
            unbonding_period,
            growth_rate,
        } => commands::update_config(deps, info, owner, unbonding_period, growth_rate),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Bonded { address } => to_binary(&queries::query_bonded(deps, address)?),
        QueryMsg::Unbonding {
            address,
            denom,
            start_after,
            limit,
        } => to_binary(&queries::query_unbonding(
            deps,
            address,
            denom,
            start_after,
            limit,
        )?),
        QueryMsg::Withdrawable { address, denom } => to_binary(&queries::query_withdrawable(
            deps,
            env.block.time,
            address,
            denom,
        )?),
        QueryMsg::Weight {
            address,
            timestamp,
            global_weight,
        } => {
            // If timestamp is not provided, use current block time
            let timestamp = timestamp.unwrap_or(env.block.time);

            // TODO: Make better timestamp handling
            to_binary(&queries::query_weight(
                deps,
                timestamp,
                address,
                global_weight,
            )?)
        }
        QueryMsg::TotalBonded {} => to_binary(&queries::query_total_bonded(deps)?),
        QueryMsg::GlobalIndex {} => to_binary(&queries::query_global_index(deps)?),
    }
}

/// Bonds the provided asset.
pub(crate) fn handle_claim_then_bond(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    bond: bool,
) -> Result<Response, ContractError> {
    // Store temp info of the sender mapped to their info and asset 
    let mut temp_info = (info.clone(), asset.clone());
    temp_info.0 = info.clone();
    temp_info.1 = asset.clone();
    TEMP_INFO.save(deps.storage, &temp_info)?;
    // Ok(Response::new().add_attribute("sender", info.sender.to_string()).add_attribute("amount", info.funds[0].amount.to_string()).add_attribute("asset", asset.to_string()))
    if bond {
    Ok(Response::new().add_submessage(SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&FeeDistributorExecuteMsg::Claim { }).unwrap(),
            funds: vec![],
        }),
        1,
    )))
}else {
    Ok(Response::new().add_submessage(SubMsg::reply_always(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_binary(&FeeDistributorExecuteMsg::Claim { }).unwrap(),
            funds: vec![],
        }),
        2,
    )))
}

}
/// The entry point to the contract for processing replies from submessages.
#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let res = handle_reply(deps, env, msg)?;

    Ok(res)
}

/// Handles the replies from the lp token and staking contract instantiation sub-messages.
pub fn handle_reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let msg_id = msg.id;
    // // parse the reply
    // let res = cw_utils::parse_reply_execute_data(msg).map_err(|_| {
    //     StdError::parse_err("MsgExecuteContractResponse", "failed to parse data")
    // })?;
    // Load the temp data 
    let temp_info = TEMP_INFO.load(deps.storage)?;
    TEMP_INFO.remove(deps.storage);
    match msg_id {
        1 => {
            let msg = to_binary(&ExecuteMsg::Bond { asset: temp_info.1 }).unwrap();
            Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute { contract_addr: env.contract.address.to_string(), msg: msg ,funds: temp_info.0.funds})))
        }
        2 => {
            let msg = to_binary(&ExecuteMsg::Unbond { asset: temp_info.1 }).unwrap();
            Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute { contract_addr: env.contract.address.to_string(), msg: msg ,funds: temp_info.0.funds})))
        }
        _ => {
            return Err(ContractError::Std(StdError::not_found(
                "reply id not found",
            )));
        }
    }
    
}


#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
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

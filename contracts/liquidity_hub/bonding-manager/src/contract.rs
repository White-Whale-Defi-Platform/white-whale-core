use cosmwasm_std::{entry_point, Addr};
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use white_whale_std::pool_network::asset;

use white_whale_std::bonding_manager::{
    Config, Epoch, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::helpers::{self, validate_growth_rate};
use crate::queries::get_expiring_epoch;
use crate::state::{BONDING_ASSETS_LIMIT, CONFIG, EPOCHS};
use crate::{commands, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-bonding_manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
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

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        pool_manager_addr: Addr::unchecked(""),
        distribution_denom: msg.distribution_denom,
        unbonding_period: msg.unbonding_period,
        growth_rate: msg.growth_rate,
        bonding_assets: msg.bonding_assets.clone(),
        grace_period: msg.grace_period,
    };

    CONFIG.save(deps.storage, &config)?;
    // Creates a new bucket for the rewards flowing from this time on, i.e. to be distributed in the next epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
    // Add a new rewards bucket for the new epoch
    EPOCHS.save(
        deps.storage,
        &0u64.to_be_bytes(),
        &Epoch {
            id: 0u64.into(),
            start_time: env.block.time,
            ..Epoch::default()
        },
    )?;
    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", config.owner.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
        ("bonding_assets", msg.bonding_assets.join(", ")),
        ("grace_period", config.grace_period.to_string()),
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
        ExecuteMsg::Bond {} => {
            let asset_to_bond = helpers::validate_funds(&deps, &info)?;
            commands::bond(
                deps,
                env.block.time,
                info.clone(),
                env,
                asset_to_bond.to_owned(),
            )
        }
        ExecuteMsg::Unbond { asset } => {
            cw_utils::nonpayable(&info)?;
            commands::unbond(deps, env.block.time, info, env, asset)
        }
        ExecuteMsg::Withdraw { denom } => {
            cw_utils::nonpayable(&info)?;
            commands::withdraw(deps, env.block.time, info.sender, denom)
        }
        ExecuteMsg::UpdateConfig {
            owner,
            pool_manager_addr,
            unbonding_period,
            growth_rate,
        } => commands::update_config(
            deps,
            info,
            owner,
            pool_manager_addr,
            unbonding_period,
            growth_rate,
        ),
        ExecuteMsg::FillRewards { .. } => commands::fill_rewards(deps, env, info),
        ExecuteMsg::FillRewardsCoin => commands::fill_rewards(deps, env, info),
        ExecuteMsg::Claim { .. } => commands::claim(deps, env, info),
        ExecuteMsg::EpochChangedHook { current_epoch } => {
            // Epoch has been updated, update rewards bucket
            // and forward the expiring epoch
            // Store epoch manager and verify the sender is him
            println!("New epoch created: {:?}", current_epoch);
            // let config = CONFIG.load(deps.storage)?;
            // let global = GLOBAL.load(deps.storage)?;

            let new_epoch_id = current_epoch.id;
            let next_epoch_id = match new_epoch_id.checked_add(1u64) {
                Some(next_epoch_id) => next_epoch_id,
                None => return Err(ContractError::Unauthorized {}),
            };
            // Creates a new bucket for the rewards flowing from this time on, i.e. to be distributed in the next epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
            // Add a new rewards bucket for the new epoch
            EPOCHS.save(
                deps.storage,
                &next_epoch_id.to_be_bytes(),
                &Epoch {
                    id: next_epoch_id.into(),
                    start_time: current_epoch.start_time.plus_days(1),
                    ..Epoch::default()
                },
            )?;
            println!("New epoch created: {}", next_epoch_id);
            // Return early if the epoch is the first one
            if new_epoch_id == 1 {
                // Creates a new bucket for the rewards flowing from this time on, i.e. to be distributed in the next epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
                // Add a new rewards bucket for the new epoch
                EPOCHS.save(
                    deps.storage,
                    &new_epoch_id.to_be_bytes(),
                    &Epoch {
                        id: next_epoch_id.into(),
                        start_time: current_epoch.start_time,
                        ..Epoch::default()
                    },
                )?;
                return Ok(Response::default()
                    .add_attributes(vec![("action", "epoch_changed_hook".to_string())]));
            }

            // forward fees from the expiring epoch to the new one.
            let mut expiring_epoch = get_expiring_epoch(deps.as_ref())?;

            if let Some(expiring_epoch) = expiring_epoch.as_mut() {
                // Load all the available assets from the expiring epoch
                let amount_to_be_forwarded = EPOCHS
                    .load(deps.storage, &expiring_epoch.id.to_be_bytes())?
                    .available;
                println!("Amount to be forwarded: {:?}", amount_to_be_forwarded);
                EPOCHS.update(
                    deps.storage,
                    &new_epoch_id.to_be_bytes(),
                    |epoch| -> StdResult<_> {
                        let mut epoch = epoch.unwrap_or_default();
                        epoch.available = asset::aggregate_coins(
                            epoch.available,
                            amount_to_be_forwarded.clone(),
                        )?;
                        epoch.total = asset::aggregate_coins(epoch.total, amount_to_be_forwarded)?;
                        Ok(epoch)
                    },
                )?;
                // Set the available assets for the expiring epoch to an empty vec now that they have been forwarded
                EPOCHS.update(
                    deps.storage,
                    &expiring_epoch.id.to_be_bytes(),
                    |epoch| -> StdResult<_> {
                        let mut epoch = epoch.unwrap_or_default();
                        epoch.available = vec![];
                        Ok(epoch)
                    },
                )?;
            }

            Ok(Response::default()
                .add_attributes(vec![("action", "epoch_changed_hook".to_string())]))
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&queries::query_config(deps)?),
        QueryMsg::Bonded { address } => to_json_binary(&queries::query_bonded(deps, address)?),
        QueryMsg::Unbonding {
            address,
            denom,
            start_after,
            limit,
        } => to_json_binary(&queries::query_unbonding(
            deps,
            address,
            denom,
            start_after,
            limit,
        )?),
        QueryMsg::Withdrawable { address, denom } => to_json_binary(&queries::query_withdrawable(
            deps,
            env.block.time,
            address,
            denom,
        )?),
        QueryMsg::Weight {
            address,
            timestamp,
            global_index,
        } => {
            // If timestamp is not provided, use current block time
            let timestamp = timestamp.unwrap_or(env.block.time);

            // TODO: Make better timestamp handling
            to_json_binary(&queries::query_weight(
                deps,
                timestamp,
                address,
                global_index,
            )?)
        }
        QueryMsg::TotalBonded {} => to_json_binary(&queries::query_total_bonded(deps)?),
        QueryMsg::GlobalIndex {} => to_json_binary(&queries::query_global_index(deps)?),
        QueryMsg::Claimable { addr } => to_json_binary(&queries::query_claimable(
            deps,
            &deps.api.addr_validate(&addr)?,
        )?),
        QueryMsg::ClaimableEpochs {} => to_json_binary(&queries::get_claimable_epochs(deps)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use semver::Version;
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

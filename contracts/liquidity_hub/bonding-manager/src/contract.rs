use cosmwasm_std::{entry_point, Addr, Uint64};
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Endian;
use semver::Version;
use white_whale_std::pool_network::asset::{self, AssetInfo};

use white_whale_std::bonding_manager::{
    Config, Epoch, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::helpers::validate_growth_rate;
use crate::state::{BONDING_ASSETS_LIMIT, CONFIG, EPOCHS};
use crate::{commands, migrations, queries};

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

    //todo since this should only accept native tokens, we could omit the asset type and pass the denom directly
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
        fee_distributor_addr: Addr::unchecked(""),
        grace_period: Uint64::new(21),
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
        ExecuteMsg::Bond { asset } => commands::bond(deps, env.block.time, info, env, asset),
        ExecuteMsg::Unbond { asset } => commands::unbond(deps, env.block.time, info, env, asset),
        ExecuteMsg::Withdraw { denom } => {
            commands::withdraw(deps, env.block.time, info.sender, denom)
        }
        ExecuteMsg::UpdateConfig {
            owner,
            unbonding_period,
            growth_rate,
            fee_distributor_addr,
        } => commands::update_config(
            deps,
            info,
            owner,
            unbonding_period,
            growth_rate,
            fee_distributor_addr,
        ),
        ExecuteMsg::FillRewards { .. } => {
            // Use aggregate_coins to get the total amount of new coins
            Ok(Response::default().add_attributes(vec![("action", "fill_rewards".to_string())]))
        }
        ExecuteMsg::Claim { .. } => commands::claim(deps, env, info),
        ExecuteMsg::EpochChangedHook { msg } => {
            // Epoch has been updated, update rewards bucket
            // and forward the expiring epoch

            let new_epoch_id = msg.current_epoch.id;
            let expiring_epoch_id = new_epoch_id.checked_sub(1u64.into()).unwrap();
            let next_epoch_id = new_epoch_id.checked_add(1u64.into()).unwrap();

            // Add a new rewards bucket for the new epoch
            // Add a new rewards bucket for the next epoch
            // Remove the rewards bucket for the expiring epoch
            // Save the next_epoch_id to the contract state

            /// Creates a new bucket for the rewards flowing from this time on, i.e. to be distributed in the next epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
            // Add a new rewards bucket for the new epoch
            EPOCHS.save(
                deps.storage,
                &next_epoch_id.to_be_bytes(),
                &Epoch {
                    id: next_epoch_id.into(),
                    start_time: msg.current_epoch.start_time,
                    ..Epoch::default()
                },
            )?;
            // Load all the available assets from the expiring epoch
            let amount_to_be_forwarded = EPOCHS
                .load(deps.storage, &expiring_epoch_id.to_be_bytes())?
                .available;
            EPOCHS.update(
                deps.storage,
                &new_epoch_id.to_be_bytes(),
                |epoch| -> StdResult<_> {
                    let mut epoch = epoch.unwrap_or_default();
                    epoch.available =
                        asset::aggregate_coins(epoch.available, amount_to_be_forwarded)?;
                    Ok(epoch)
                },
            )?;
            // Set the available assets for the expiring epoch to an empty vec now that they have been forwarded
            EPOCHS.update(
                deps.storage,
                &expiring_epoch_id.to_be_bytes(),
                |epoch| -> StdResult<_> {
                    let mut epoch = epoch.unwrap_or_default();
                    epoch.available = vec![];
                    Ok(epoch)
                },
            )?;

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
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
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
    Ok(Response::default())
}

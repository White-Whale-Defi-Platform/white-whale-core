use crate::error::ContractError;
use crate::helpers::{self, validate_growth_rate};
use crate::state::{BONDING_ASSETS_LIMIT, CONFIG, UPCOMING_REWARD_BUCKET};
use crate::{bonding, commands, queries, rewards};
use cosmwasm_std::{ensure, entry_point, Addr, Reply};
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::{get_contract_version, set_contract_version};
use white_whale_std::bonding_manager::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, UpcomingRewardBucket,
};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-bonding_manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const LP_WITHDRAWAL_REPLY_ID: u64 = 0;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    ensure!(
        msg.bonding_assets.len() <= BONDING_ASSETS_LIMIT,
        ContractError::InvalidBondingAssetsLimit(BONDING_ASSETS_LIMIT, msg.bonding_assets.len(),)
    );

    validate_growth_rate(msg.growth_rate)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        pool_manager_addr: Addr::unchecked(""),
        epoch_manager_addr: Addr::unchecked(""),
        distribution_denom: msg.distribution_denom,
        unbonding_period: msg.unbonding_period,
        growth_rate: msg.growth_rate,
        bonding_assets: msg.bonding_assets.clone(),
        grace_period: msg.grace_period,
    };

    CONFIG.save(deps.storage, &config)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, Some(info.sender.as_str()))?;

    // Initialize the upcoming reward bucket
    UPCOMING_REWARD_BUCKET.save(deps.storage, &UpcomingRewardBucket::default())?;

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", info.sender.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
        ("bonding_assets", msg.bonding_assets.join(", ")),
        ("grace_period", config.grace_period.to_string()),
    ]))
}

// Reply entrypoint handling LP withdraws from fill_rewards
#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        LP_WITHDRAWAL_REPLY_ID => rewards::commands::handle_lp_withdrawal_reply(deps, msg),
        _ => Err(ContractError::Unauthorized),
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
        ExecuteMsg::Bond => {
            let asset_to_bond = helpers::validate_funds(&deps, &info)?;
            bonding::commands::bond(deps, info, env, asset_to_bond)
        }
        ExecuteMsg::Unbond { asset } => {
            cw_utils::nonpayable(&info)?;
            bonding::commands::unbond(deps, info, env, asset)
        }
        ExecuteMsg::Withdraw { denom } => {
            cw_utils::nonpayable(&info)?;
            bonding::commands::withdraw(deps, info.sender, denom)
        }
        ExecuteMsg::UpdateConfig {
            epoch_manager_addr,
            pool_manager_addr,
            unbonding_period,
            growth_rate,
        } => {
            cw_utils::nonpayable(&info)?;
            commands::update_config(
                deps,
                info,
                epoch_manager_addr,
                pool_manager_addr,
                unbonding_period,
                growth_rate,
            )
        }
        ExecuteMsg::FillRewards => rewards::commands::fill_rewards(deps, env, info),
        ExecuteMsg::Claim => rewards::commands::claim(deps, info),
        ExecuteMsg::EpochChangedHook { current_epoch } => {
            rewards::commands::on_epoch_created(deps, info, current_epoch)
        }
        ExecuteMsg::UpdateOwnership(action) => {
            cw_utils::nonpayable(&info)?;
            white_whale_std::common::update_ownership(deps, env, info, action).map_err(Into::into)
        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config => Ok(to_json_binary(&queries::query_config(deps)?)?),
        QueryMsg::Bonded { address } => Ok(to_json_binary(&queries::query_bonded(deps, address)?)?),
        QueryMsg::Unbonding {
            address,
            denom,
            start_after,
            limit,
        } => Ok(to_json_binary(&queries::query_unbonding(
            deps,
            address,
            denom,
            start_after,
            limit,
        )?)?),
        QueryMsg::Withdrawable { address, denom } => Ok(to_json_binary(
            &queries::query_withdrawable(deps, address, denom)?,
        )?),
        // QueryMsg::Weight {
        //     address,
        //     epoch_id,
        //     global_index,
        // } => {
        //     let epoch_id = if let Some(epoch_id) = epoch_id {
        //         epoch_id
        //     } else {
        //         // If epoch_id is not provided, use current epoch
        //         let config = CONFIG.load(deps.storage)?;
        //         let current_epoch: white_whale_std::epoch_manager::epoch_manager::EpochResponse =
        //             deps.querier.query_wasm_smart(
        //                 config.epoch_manager_addr,
        //                 &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        //             )?;
        //         current_epoch.epoch.id
        //     };
        //
        //     Ok(to_json_binary(&queries::query_weight(
        //         deps,
        //         epoch_id,
        //         address,
        //         global_index,
        //     )?)?)
        // }
        QueryMsg::GlobalIndex { epoch_id } => Ok(to_json_binary(&queries::query_global_index(
            deps, epoch_id,
        )?)?),
        QueryMsg::Claimable { address } => {
            Ok(to_json_binary(&queries::query_claimable(&deps, address)?)?)
        }
        QueryMsg::Ownership {} => Ok(to_json_binary(&cw_ownable::get_ownership(deps.storage)?)?),
        QueryMsg::Rewards { address } => {
            Ok(to_json_binary(&queries::query_rewards(deps, address)?)?)
        }
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

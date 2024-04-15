use cosmwasm_std::{ensure, entry_point, Coin, CosmosMsg, Order};
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{get_contract_version, set_contract_version};
use cw_utils::PaymentError;
use white_whale_std::lp_common::LP_SYMBOL;
use white_whale_std::pool_network::asset;

use white_whale_std::bonding_manager::{
    Config, Epoch, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg,
};

use crate::error::ContractError;
use crate::helpers::validate_growth_rate;
use crate::queries::get_expiring_epoch;
use crate::state::{BONDING_ASSETS_LIMIT, CONFIG, EPOCHS};
use crate::{commands, queries};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-bonding_manager";
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

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        unbonding_period: msg.unbonding_period,
        growth_rate: msg.growth_rate,
        bonding_assets: msg.bonding_assets.clone(),
        grace_period: msg.grace_period,
    };

    CONFIG.save(deps.storage, &config)?;

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
            let config = CONFIG.load(deps.storage)?;
            // Ensure that the user has sent some funds
            ensure!(!info.funds.is_empty(), PaymentError::NoFunds {});
            let asset_to_bond = {
                // Filter the funds to include only those with accepted denominations
                let valid_funds: Vec<&Coin> = info
                    .funds
                    .iter()
                    .filter(|coin| config.bonding_assets.contains(&coin.denom))
                    .collect();

                // Check if there are no valid funds after filtering
                if valid_funds.is_empty() {
                    Err(PaymentError::MissingDenom("test".to_string()))
                } else if valid_funds.len() == 1 {
                    // If exactly one valid fund is found, return the amount
                    Ok(valid_funds[0])
                } else {
                    // If multiple valid denominations are found (which shouldn't happen), return an error
                    Err(PaymentError::MultipleDenoms {})
                }
            }?;

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
            unbonding_period,
            growth_rate,
        } => commands::update_config(deps, info, owner, unbonding_period, growth_rate),
        ExecuteMsg::FillRewards { .. } => {
            // Use aggregate_coins to get the total amount of new coins
            // Finding the most recent EpochID
            let most_recent_epoch_id = EPOCHS
                .keys(deps.storage, None, None, Order::Descending)
                .next()
                .unwrap()?;

            let _messages: Vec<CosmosMsg> = vec![];
            // Verify coins are coming
            // swap non-whale to whale
            // Search info funds for LP tokens, LP tokens will contain LP_SYMBOL from lp_common and the string .pair.
            let lp_tokens = info
                .funds
                .iter()
                .filter(|coin| coin.denom.contains(".pair.") | coin.denom.contains(LP_SYMBOL));
            // LP tokens have the format "{pair_label}.pair.{identifier}.{LP_SYMBOL}", get the identifier and not the LP SYMBOL
            let _pair_identifier = lp_tokens
                .map(|coin| coin.denom.split(".pair.").collect::<Vec<&str>>()[1])
                .next()
                .unwrap();

            // // if LP Tokens ,verify and withdraw then swap to whale
            // let lp_withdrawal_msg = white_whale_std::pool_manager::ExecuteMsg::WithdrawLiquidity { pair_identifier: pair_identifier.to_string() };
            // messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            //     contract_addr: ,
            //     msg: to_json_binary(&lp_withdrawal_msg)?,
            //     funds: vec![],
            // }));

            // Note: Might need to convert back to ints and use that for ranking to get the most recent ID
            // Note: After swap,
            EPOCHS.update(
                deps.storage,
                &most_recent_epoch_id,
                |bucket| -> StdResult<_> {
                    let mut bucket = bucket.unwrap_or_default();
                    bucket.available = asset::aggregate_coins(bucket.available, vec![])?;
                    Ok(bucket)
                },
            )?;
            Ok(Response::default().add_attributes(vec![("action", "fill_rewards".to_string())]))
        }
        ExecuteMsg::Claim { .. } => commands::claim(deps, env, info),
        ExecuteMsg::EpochChangedHook { msg } => {
            // Epoch has been updated, update rewards bucket
            // and forward the expiring epoch
            // Store epoch manager and verify the sender is him

            let new_epoch_id = msg.current_epoch.id;
            let expiring_epoch_id = new_epoch_id.checked_sub(1u64).unwrap();
            let next_epoch_id = new_epoch_id.checked_add(1u64).unwrap();
            // Verify that it is indeed the expiring epoch that is being forwarded
            let _ = match get_expiring_epoch(deps.as_ref())? {
                Some(epoch) if epoch.id.u64() == expiring_epoch_id => Ok(()),
                Some(_) => Err(ContractError::Unauthorized {}),
                None => Err(ContractError::Unauthorized {}), // Handle the case where there is no expiring epoch
            };

            // Creates a new bucket for the rewards flowing from this time on, i.e. to be distributed in the next epoch. Also, forwards the expiring epoch (only 21 epochs are live at a given moment)
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
                    epoch.total = epoch.available.clone();
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
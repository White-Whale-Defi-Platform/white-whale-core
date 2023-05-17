use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, QueryRequest, ReplyOn, Response, StdError,
    SubMsg, Timestamp, Uint64, WasmMsg, WasmQuery, Uint128, Decimal,
};

use white_whale::fee_distributor::{Epoch, EpochConfig};
use white_whale::pool_network::asset;
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::whale_lair::{BondingWeightResponse, QueryMsg, GlobalIndex};

use crate::contract::EPOCH_CREATION_REPLY_ID;
use crate::helpers::{validate_epoch_config, validate_grace_period};
use crate::state::{get_current_epoch, query_claimable, CONFIG, EPOCHS, LAST_CLAIMED_EPOCH};
use crate::ContractError;

/// Creates a new epoch, forwarding available tokens from epochs that are past the grace period.
pub fn create_new_epoch(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_epoch = get_current_epoch(deps.as_ref())?.epoch;
    if env
        .block
        .time
        .minus_nanos(current_epoch.start_time.nanos())
        .nanos()
        < config.epoch_config.duration.u64()
    {
        return Err(ContractError::CurrentEpochNotExpired {});
    }

    let start_time =
        if current_epoch.id == Uint64::zero() && current_epoch.start_time == Timestamp::default() {
            // if it's the very first epoch, set the start time to the genesis epoch
            let genesis_epoch_timestamp =
                Timestamp::from_nanos(config.epoch_config.genesis_epoch.u64());

            if env.block.time.nanos() < genesis_epoch_timestamp.nanos() {
                return Err(ContractError::GenesisEpochNotStarted {});
            }

            genesis_epoch_timestamp
        } else {
            current_epoch
                .start_time
                .plus_nanos(config.epoch_config.duration.u64())
        };
    
    // Query the current global index
    // let global_index: GlobalIndex = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
    //     contract_addr: config.bonding_contract_addr.to_string(),
    //     msg: to_binary(&QueryMsg::GlobalIndex {})?,
    // }))?;
    
    let new_epoch = Epoch {
        id: current_epoch.id.checked_add(Uint64::new(1u64))?,
        start_time,
        total: vec![],
        available: vec![],
        claimed: vec![],
        weight: Uint128::zero(),
    };

    Ok(Response::new()
        .add_submessage(SubMsg {
            id: EPOCH_CREATION_REPLY_ID,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.fee_collector_addr.to_string(),
                msg: to_binary(&white_whale::fee_collector::ExecuteMsg::ForwardFees {
                    epoch: new_epoch.clone(),
                    forward_fees_as: config.distribution_asset,
                })?,
                funds: vec![],
            }),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        })
        .add_attributes(vec![
            ("action", "new_epoch".to_string()),
            ("new_epoch", new_epoch.id.to_string()),
        ]))
}

/// Claims pending rewards for the sender.
pub fn claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Query the fee share of the sender based on the ratio of his weight and the global weight at the current moment
    let config = CONFIG.load(deps.storage)?;
    

    let claimable_epochs = query_claimable(deps.as_ref(), &info.sender)?.epochs;
    if claimable_epochs.is_empty() {
        return Err(ContractError::NothingToClaim {});
    }

    let mut claimable_fees = vec![];
    for mut epoch in claimable_epochs.clone() {
        let bonding_weight_response: BondingWeightResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: config.bonding_contract_addr.to_string(),
                msg: to_binary(&QueryMsg::Weight {
                    address: info.sender.to_string(),
                    timestamp: Some(epoch.start_time),
                    global_weight: Some(epoch.weight),
                })?,
            }))?;
    
        println!("bonding_weight_response.weight: {}", bonding_weight_response.weight);
        println!("bonding: {:?}", bonding_weight_response);
        println!("epoch.weight: {}", epoch.weight);
        println!("epoch: {:?}", epoch);

        for fee in epoch.total.iter() {
            let reward = fee.amount * bonding_weight_response.share;

            // make sure the reward is sound
            let _ = epoch
                .available
                .iter()
                .find(|available_fee| available_fee.info == fee.info)
                .map(|available_fee| {
                    if reward > available_fee.amount {
                        return Err(ContractError::InvalidReward {});
                    }
                    Ok(())
                })
                .ok_or_else(|| StdError::generic_err("Invalid fee"))?;

            // add the reward to the claimable fees
            claimable_fees = asset::aggregate_assets(
                claimable_fees,
                vec![Asset {
                    info: fee.info.clone(),
                    amount: reward,
                }],
            )?;

            // modify the epoch to reflect the new available and claimed amount
            for available_fee in epoch.available.iter_mut() {
                if available_fee.info == fee.info {
                    available_fee.amount = available_fee.amount.checked_sub(reward)?;
                }
            }

            if epoch.claimed.is_empty() {
                epoch.claimed = vec![Asset {
                    info: fee.info.clone(),
                    amount: reward,
                }];
            } else {
                for claimed_fee in epoch.claimed.iter_mut() {
                    if claimed_fee.info == fee.info {
                        claimed_fee.amount = claimed_fee.amount.checked_add(reward)?;
                    }
                }
            }

            EPOCHS.save(deps.storage, &epoch.id.to_be_bytes(), &epoch)?;
        }
    }

    // update the last claimed epoch for the user
    LAST_CLAIMED_EPOCH.save(deps.storage, &info.sender, &claimable_epochs[0].id)?;

    // send funds to the user
    let mut messages = vec![];
    for fee in claimable_fees {
        messages.push(fee.into_msg(info.sender.clone())?);
    }

    Ok(Response::new()
        .add_attributes(vec![("action", "claim")])
        .add_messages(messages))
}

/// Updates the [Config] of the contract
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    bonding_contract_addr: Option<String>,
    fee_collector_addr: Option<String>,
    grace_period: Option<Uint64>,
    distribution_asset: Option<AssetInfo>,
    epoch_config: Option<EpochConfig>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        config.owner = deps.api.addr_validate(&owner)?;
    }

    if let Some(bonding_contract_addr) = bonding_contract_addr {
        config.bonding_contract_addr = deps.api.addr_validate(&bonding_contract_addr)?;
    }

    if let Some(fee_collector_addr) = fee_collector_addr {
        config.fee_collector_addr = deps.api.addr_validate(&fee_collector_addr)?;
    }

    if let Some(distribution_asset) = distribution_asset {
        config.distribution_asset = distribution_asset;
    }

    if let Some(epoch_config) = epoch_config {
        validate_epoch_config(&epoch_config)?;
        config.epoch_config = epoch_config;
    }

    if let Some(grace_period) = grace_period {
        validate_grace_period(&grace_period)?;

        if grace_period < config.grace_period {
            return Err(ContractError::GracePeriodDecrease {});
        }

        config.grace_period = grace_period;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "update_config".to_string()),
        ("owner", config.owner.to_string()),
        (
            "bonding_contract_addr",
            config.bonding_contract_addr.to_string(),
        ),
        ("fee_collector_addr", config.fee_collector_addr.to_string()),
        ("grace_period", config.grace_period.to_string()),
        ("distribution_asset", config.distribution_asset.to_string()),
        ("epoch_config", config.epoch_config.to_string()),
    ]))
}

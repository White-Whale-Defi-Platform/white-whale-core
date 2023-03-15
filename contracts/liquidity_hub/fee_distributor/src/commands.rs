use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, MessageInfo, QueryRequest, ReplyOn, Response, StdError,
    SubMsg, Timestamp, Uint64, WasmMsg, WasmQuery,
};

use white_whale::fee_distributor::Epoch;
use white_whale::fee_distributor::ExecuteMsg;
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::whale_lair::{BondingWeightResponse, QueryMsg};

use crate::contract::EPOCH_CREATION_REPLY_ID;
use crate::helpers::validate_grace_period;
use crate::state::{
    get_claimable_epochs, get_current_epoch, get_epoch, CONFIG, EPOCHS, LAST_CLAIMED_EPOCH,
};
use crate::{helpers, ContractError};

/// Creates a new epoch, forwarding available tokens from epochs that are past the grace period.
pub fn create_new_epoch(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let current_epoch = get_current_epoch(deps.as_ref())?.epoch;

    if env
        .block
        .time
        .minus_seconds(current_epoch.start_time.seconds())
        .seconds()
        < config.epoch_config.duration.u64()
    {
        return Err(ContractError::CurrentEpochNotExpired {});
    }

    let start_time =
        if current_epoch.id == Uint64::zero() && current_epoch.start_time == Timestamp::default() {
            // if it's the very first epoch, set the start time to the genesis epoch
            Timestamp::from_seconds(config.epoch_config.genesis_epoch.u64())
        } else {
            current_epoch
                .start_time
                .plus_seconds(config.epoch_config.duration.u64())
        };

    let new_epoch = Epoch {
        id: current_epoch.id.checked_add(Uint64::new(1u64))?,
        start_time,
        total: vec![],
        available: vec![],
        claimed: vec![],
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
            ("action", "create_new_epoch".to_string()),
            ("new_epoch", new_epoch.id.to_string()),
        ]))
}

pub fn claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Query the fee share of the sender based on the ratio of his weight and the global weight at the current moment
    let config = CONFIG.load(deps.storage)?;
    let bonding_weight_response: BondingWeightResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.bonding_contract_addr.to_string(),
            msg: to_binary(&QueryMsg::Weight {
                address: info.sender.to_string(),
            })?,
        }))?;

    let fee_share = bonding_weight_response.share;

    let mut claimable_epochs = get_claimable_epochs(deps.as_ref())?.epochs;
    let last_claimed_epoch = LAST_CLAIMED_EPOCH.may_load(deps.storage, &info.sender)?;

    // filter out epochs that have already been claimed by the user
    if let Some(last_claimed_epoch) = last_claimed_epoch {
        claimable_epochs.retain(|epoch| epoch.id > last_claimed_epoch);

        // the user has already claimed fees on all claimable epochs
        if claimable_epochs.is_empty() {
            return Err(ContractError::NothingToClaim {});
        }
    };

    let mut claimable_fees = vec![];
    for mut epoch in claimable_epochs.clone() {
        for fee in epoch.total.iter() {
            let reward = fee.amount * fee_share;

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
            claimable_fees = helpers::aggregate_fees(
                claimable_fees,
                vec![Asset {
                    info: fee.info.clone(),
                    amount: reward,
                }],
            );

            // modify the epoch to reflect the new available and claimed amount
            for available_fee in epoch.available.iter_mut() {
                if available_fee.info == fee.info {
                    available_fee.amount = available_fee.amount.checked_sub(reward)?;
                }
            }

            for claimed_fee in epoch.claimed.iter_mut() {
                if claimed_fee.info == fee.info {
                    claimed_fee.amount = claimed_fee.amount.checked_add(reward)?;
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
    env: Env,
    owner: Option<String>,
    bonding_contract_addr: Option<String>,
    fee_collector_addr: Option<String>,
    grace_period: Option<Uint64>,
    distribution_asset: Option<AssetInfo>,
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

    let mut messages = vec![];
    if let Some(grace_period) = grace_period {
        validate_grace_period(&grace_period)?;

        if grace_period < config.grace_period {
            // if the grace period is smaller than the current one, it means the fees from the epochs
            // that are expiring need to be forwarded to a new epoch.
            // Create a new epoch, and then refill it with the fees from the expired epochs.

            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::NewEpoch {})?,
                funds: vec![],
            }));

            let claimable_epochs = get_claimable_epochs(deps.as_ref())?.epochs;
            let (_, expired_epochs) = claimable_epochs.split_at(grace_period.u64() as usize);

            let mut forwarding_assets = vec![];
            for epoch in expired_epochs {
                forwarding_assets =
                    helpers::aggregate_fees(forwarding_assets, epoch.available.clone());
            }

            // the new epoch's id is the current epoch's id + 1. Refill that epoch with the fees from
            // expiring epochs.
            let new_epoch_id = get_current_epoch(deps.as_ref())?
                .epoch
                .id
                .checked_add(Uint64::one())?;
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::RefillEpoch {
                    epoch_id: new_epoch_id,
                    fees: forwarding_assets,
                })?,
                funds: vec![],
            }));
        }

        config.grace_period = grace_period;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "update_config".to_string()),
        ("owner", config.owner.to_string()),
        (
            "bonding_contract_addr",
            config.bonding_contract_addr.to_string(),
        ),
        ("fee_collector_addr", config.fee_collector_addr.to_string()),
        ("grace_period", config.grace_period.to_string()),
        ("distribution_asset", config.distribution_asset.to_string()),
    ]))
}

/// Refills the epoch with the given fees. This is only possible to do iff the fees in the epoch have
/// not been claimed.
pub fn refill_epoch(
    deps: DepsMut,
    info: MessageInfo,
    epoch_id: Uint64,
    fees: Vec<Asset>,
) -> Result<Response, ContractError> {
    // make sure the fees match the funds sent
    if info
        .funds
        .iter()
        .map(|coin| Asset {
            info: AssetInfo::NativeToken {
                denom: coin.denom.clone(),
            },
            amount: coin.amount,
        })
        .any(|asset| !fees.contains(&asset))
    {
        return Err(ContractError::AssetMismatch {});
    }

    let mut epoch = get_epoch(deps.as_ref(), epoch_id)?.epoch;

    if epoch.validate_refillable() {
        let fees = helpers::aggregate_fees(epoch.available, fees);
        epoch.available = fees.clone();
        epoch.total = fees;

        EPOCHS.save(deps.storage, &epoch_id.to_be_bytes(), &epoch)?;

        return Ok(Response::new().add_attributes(vec![
            ("action", "refill_epoch".to_string()),
            ("epoch_id", epoch_id.to_string()),
        ]));
    }

    Err(ContractError::CannotRefillEpoch(epoch_id))
}

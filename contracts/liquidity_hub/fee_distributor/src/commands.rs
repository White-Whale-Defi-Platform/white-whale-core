use cosmwasm_std::{DepsMut, MessageInfo, Response, StdError};

use terraswap::asset::{Asset, AssetInfo};

use crate::msg::ExecuteMsg;
use crate::state::{get_current_epoch, get_expiring_epoch, Epoch, CONFIG, EPOCHS};
use crate::ContractError;

/// Creates a new epoch, forwarding available tokens from epochs that are past the grace period.
pub fn create_new_epoch(
    deps: DepsMut,
    info: MessageInfo,
    mut fees: Vec<Asset>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // only the fee collector can call this function
    if info.sender != config.fee_collector_addr {
        return Err(ContractError::Unauthorized {});
    }

    // make sure the fees match the funds sent
    let invalid_funds: Vec<Asset> = info
        .funds
        .iter()
        .map(|coin| Asset {
            info: AssetInfo::NativeToken {
                denom: coin.denom.clone(),
            },
            amount: coin.amount,
        })
        .filter(|asset| !fees.contains(asset))
        .collect();
    if !invalid_funds.is_empty() {
        return Err(ContractError::AssetMismatch {});
    }

    // forward fees from previous epoch to the new one
    let current_epoch = get_current_epoch(deps.as_ref())?;
    let expiring_epoch = get_expiring_epoch(deps.as_ref())?;
    let unclaimed_fees = expiring_epoch
        .map(|epoch| epoch.available)
        .unwrap_or(vec![]);

    fees = aggregate_fees(fees, unclaimed_fees);

    let new_epoch = Epoch {
        id: current_epoch
            .id
            .checked_add(1)
            .ok_or(StdError::generic_err("couldn't compute epoch id"))?,
        total: fees.clone(),
        available: fees.clone(),
        claimed: vec![],
    };

    EPOCHS.save(deps.storage, &new_epoch.id.to_be_bytes(), &new_epoch)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "create_new_epoch".to_string()),
        ("new_epoch", new_epoch.id.to_string()),
        ("fees_to_distribute", format!("{:?}", fees)),
    ]))
}

/// Aggregates the new fees to be distributed with the ones unclaimed from another epoch.
fn aggregate_fees(fees: Vec<Asset>, unclaimed_fees: Vec<Asset>) -> Vec<Asset> {
    let mut aggregated_fees = fees;

    for fee in unclaimed_fees {
        let mut found = false;
        for mut aggregated_fee in &mut aggregated_fees {
            if fee.info == aggregated_fee.info {
                println!("adding amounts: {} + {}", fee.amount, aggregated_fee.amount);
                aggregated_fee.amount += fee.amount;
                found = true;
                break;
            }
        }

        if !found {
            aggregated_fees.push(fee);
        }
    }

    aggregated_fees
}

pub fn claim(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    /*
    // What percentage does the sender get based on the ratio of his weight and the global weight at the current moment. Since people might be staking/unstaking during the epoch, the amount of claimable fees fluctuates.
    fee_share = stakingContract[sender].weight / stakingContract.weight
    for epoch in Epochs[CurrentEpoch-gracePeriod:]:
    // Break if user already claimed on the epoch
    if LastClaimed[sender] >= epoch.value:
    break
        rewards = epoch.total / fee_share // For every claimable assset, calculate the tokens you get
    Send fees to sender
    epoch.available -= rewards // Subtract tokens sent to sender from available tokens
    LastClaimed[sender].epoch = epoch // Increase last claimed epoch by user to prevent double claim.
    */

    Ok(Response::new().add_attributes(vec![("action", "claim")]))
}

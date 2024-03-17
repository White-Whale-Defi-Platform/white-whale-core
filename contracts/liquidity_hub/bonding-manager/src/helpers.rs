use cosmwasm_std::{Coin, Decimal, DepsMut, Env, MessageInfo, StdResult, Timestamp, Uint64};
use white_whale_std::epoch_manager::epoch_manager::EpochConfig;
use white_whale_std::fee_distributor::{ClaimableEpochsResponse, EpochResponse};
use white_whale_std::pool_network::asset::{Asset, AssetInfo};

use crate::error::ContractError;
use crate::state::CONFIG;

/// Validates that the growth rate is between 0 and 1.
pub fn validate_growth_rate(growth_rate: Decimal) -> Result<(), ContractError> {
    if growth_rate > Decimal::percent(100) {
        return Err(ContractError::InvalidGrowthRate {});
    }
    Ok(())
}

/// Validates that the asset sent on the message matches the asset provided and is whitelisted for bonding.
pub fn validate_funds(
    deps: &DepsMut,
    info: &MessageInfo,
    asset: &Coin,
    denom: String,
) -> Result<(), ContractError> {
    let bonding_assets = CONFIG.load(deps.storage)?.bonding_assets;

    if info.funds.len() != 1
        || info.funds[0].amount.is_zero()
        || info.funds[0].amount != asset.amount
        || info.funds[0].denom != denom
        || !bonding_assets.iter().any(|asset_info| {
            let d = match asset_info {
                AssetInfo::NativeToken { denom } => denom.clone(),
                AssetInfo::Token { .. } => String::new(),
            };
            d == denom
        })
    {
        return Err(ContractError::AssetMismatch {});
    }

    Ok(())
}

/// if user has unclaimed rewards, fail with an exception prompting them to claim
pub fn validate_claimed(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
    // Query fee distributor
    // if user has unclaimed rewards, fail with an exception prompting them to claim
    let config = CONFIG.load(deps.storage)?;
    let fee_distributor = config.fee_distributor_addr;

    // Do a smart query for Claimable
    let claimable_rewards: ClaimableEpochsResponse = deps.querier.query_wasm_smart(
        fee_distributor,
        &white_whale_std::fee_distributor::QueryMsg::Claimable {
            address: info.sender.to_string(),
        },
    )?;

    // If epochs is greater than none
    if !claimable_rewards.epochs.is_empty() {
        return Err(ContractError::UnclaimedRewards {});
    }

    Ok(())
}

/// Validates that the current time is not more than a day after the epoch start time. Helps preventing
/// global_index timestamp issues when querying the weight.
pub fn validate_bonding_for_current_epoch(deps: &DepsMut, env: &Env) -> Result<(), ContractError> {
    // Query current epoch on fee distributor
    let config = CONFIG.load(deps.storage)?;
    let fee_distributor = config.fee_distributor_addr;

    let epoch_response: EpochResponse = deps.querier.query_wasm_smart(
        fee_distributor,
        &white_whale_std::fee_distributor::QueryMsg::CurrentEpoch {},
    )?;

    let current_epoch = epoch_response.epoch;
    let current_time = env.block.time.seconds();
    pub const DAY_IN_SECONDS: u64 = 86_400u64;

    // if the current time is more than a day after the epoch start time, then it means the latest
    // epoch has not been created and thus, prevent users from bonding/unbonding to avoid global_index
    // timestamp issues when querying the weight.
    if current_epoch.id != Uint64::zero()
        && current_time - current_epoch.start_time.seconds() > DAY_IN_SECONDS
    {
        return Err(ContractError::NewEpochNotCreatedYet {});
    }

    Ok(())
}

/// Calculates the epoch id for any given timestamp based on the genesis epoch configuration.
pub fn calculate_epoch(
    genesis_epoch_config: EpochConfig,
    timestamp: Timestamp,
) -> StdResult<Uint64> {
    let epoch_duration: Uint64 = genesis_epoch_config.duration;

    // if this is true, it means the epoch is before the genesis epoch. In that case return Epoch 0.
    if Uint64::new(timestamp.nanos()) < genesis_epoch_config.genesis_epoch {
        return Ok(Uint64::zero());
    }

    let elapsed_time =
        Uint64::new(timestamp.nanos()).checked_sub(genesis_epoch_config.genesis_epoch)?;
    let epoch = elapsed_time
        .checked_div(epoch_duration)?
        .checked_add(Uint64::one())?;

    Ok(epoch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_epoch() {
        let genesis_epoch = EpochConfig {
            duration: Uint64::from(86400000000000u64), // 1 day in nanoseconds
            genesis_epoch: Uint64::from(1683212400000000000u64), // May 4th 2023 15:00:00
        };

        // First bond timestamp equals genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683212400000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(1u64));

        // First bond timestamp is one day after genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683309600000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(2u64));

        // First bond timestamp is three days after genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683471600000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(4u64));

        // First bond timestamp is before genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683212300000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::zero());

        // First bond timestamp is within the same epoch as genesis epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683223200000000000u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(1u64));

        // First bond timestamp is at the end of the genesis epoch, but not exactly (so it's still not epoch 2)
        let first_bond_timestamp = Timestamp::from_nanos(1683298799999999999u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(1u64));

        // First bond timestamp is exactly one nanosecond after the end of an epoch
        let first_bond_timestamp = Timestamp::from_nanos(1683298800000000001u64);
        let epoch = calculate_epoch(genesis_epoch.clone(), first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(2u64));

        // First bond timestamp is June 13th 2023 10:56:53
        let first_bond_timestamp = Timestamp::from_nanos(1686653813000000000u64);
        let epoch = calculate_epoch(genesis_epoch, first_bond_timestamp).unwrap();
        assert_eq!(epoch, Uint64::from(40u64));
    }
}

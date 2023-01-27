use cosmwasm_std::{Addr, DepsMut, StdError, Uint128};
use cw_storage_plus::{Item, Map};

use white_whale::whale_lair::{Config, GlobalIndex, Stake};

use crate::ContractError;

pub const CONFIG: Item<Config> = Item::new("config");
pub const STAKE: Map<&Addr, Stake> = Map::new("stake");
pub const UNSTAKE: Map<(&[u8], u64), Stake> = Map::new("unstake");
pub const GLOBAL: Item<GlobalIndex> = Item::new("global");

/// Updates the local weight of the given address.
pub fn update_local_weight(
    deps: &mut DepsMut,
    address: Addr,
    block_height: u64,
    mut stake: Stake,
) -> Result<Stake, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    stake.weight += stake
        .amount
        .checked_mul(Uint128::from(config.growth_rate))?
        .checked_mul(Uint128::from(
            block_height
                .checked_sub(stake.block_height)
                .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
        ))?;

    stake.block_height = block_height;

    STAKE.save(deps.storage, &address, &stake)?;

    Ok(stake)
}

/// Updates the global weight of the contract.
pub fn update_global_weight(
    deps: &mut DepsMut,
    block_height: u64,
    mut global_index: GlobalIndex,
) -> Result<GlobalIndex, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    global_index.weight += global_index
        .stake
        .checked_mul(Uint128::from(config.growth_rate))?
        .checked_mul(Uint128::from(
            block_height
                .checked_sub(global_index.block_height)
                .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
        ))?;
    global_index.block_height = block_height;

    GLOBAL.save(deps.storage, &global_index)?;

    Ok(global_index)
}

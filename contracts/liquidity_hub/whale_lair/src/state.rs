use cosmwasm_std::{Addr, DepsMut, StdError, Uint128};
use cw_storage_plus::{Item, Map};

use white_whale::whale_lair::{Bond, Config, GlobalIndex};

use crate::ContractError;

type BlockHeight = u64;

pub const CONFIG: Item<Config> = Item::new("config");
pub const BOND: Map<&Addr, Bond> = Map::new("bond");
pub const UNBOND: Map<(&Addr, BlockHeight), Bond> = Map::new("unbond");
pub const GLOBAL: Item<GlobalIndex> = Item::new("global");

/// Updates the local weight of the given address.
pub fn update_local_weight(
    deps: &mut DepsMut,
    address: Addr,
    block_height: u64,
    mut bond: Bond,
) -> Result<Bond, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    bond.weight = bond.weight.checked_add(
        bond.amount
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                block_height
                    .checked_sub(bond.block_height)
                    .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
            ))?,
    )?;

    bond.block_height = block_height;

    BOND.save(deps.storage, &address, &bond)?;

    Ok(bond)
}

/// Updates the global weight of the contract.
pub fn update_global_weight(
    deps: &mut DepsMut,
    block_height: u64,
    mut global_index: GlobalIndex,
) -> Result<GlobalIndex, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    global_index.weight = global_index.weight.checked_add(
        global_index
            .bond
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                block_height
                    .checked_sub(global_index.block_height)
                    .ok_or_else(|| StdError::generic_err("Invalid block height"))?,
            ))?,
    )?;

    global_index.block_height = block_height;

    GLOBAL.save(deps.storage, &global_index)?;

    Ok(global_index)
}

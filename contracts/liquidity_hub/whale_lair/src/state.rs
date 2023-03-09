use cosmwasm_std::{Addr, DepsMut, StdError, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use white_whale::whale_lair::{AssetInfo, Bond, Config, GlobalIndex};

use crate::ContractError;

type BlockHeight = u64;
type Denom = str;

pub const BONDING_ASSETS_LIMIT: usize = 2;
pub const CONFIG: Item<Config> = Item::new("config");
pub const BOND: Map<(&Addr, &Denom), Bond> = Map::new("bond");
pub const UNBOND: Map<(&Addr, &Denom, u64), Bond> = Map::new("unbond");
pub const GLOBAL: Item<GlobalIndex> = Item::new("global");

/// Updates the local weight of the given address.
pub fn update_local_weight(
    deps: &mut DepsMut,
    address: Addr,
    timestamp: Timestamp,
    mut bond: Bond,
) -> Result<Bond, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    bond.weight = bond.weight.checked_add(
        bond.asset
            .amount
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                timestamp.minus_seconds(bond.timestamp.seconds()).seconds(),
            ))?,
    )?;

    bond.timestamp = timestamp;

    let denom: &String = match &bond.asset.info {
        AssetInfo::Token { .. } => return Err(ContractError::AssetMismatch {}),
        AssetInfo::NativeToken { denom } => denom,
    };

    BOND.save(deps.storage, (&address, denom), &bond)?;

    Ok(bond)
}

/// Updates the global weight of the contract.
pub fn update_global_weight(
    deps: &mut DepsMut,
    timestamp: Timestamp,
    mut global_index: GlobalIndex,
) -> Result<GlobalIndex, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    global_index.weight = global_index.weight.checked_add(
        global_index
            .bond_amount
            .checked_mul(Uint128::from(config.growth_rate))?
            .checked_mul(Uint128::from(
                timestamp
                    .minus_seconds(global_index.timestamp.seconds())
                    .seconds(),
            ))?,
    )?;

    global_index.timestamp = timestamp;

    GLOBAL.save(deps.storage, &global_index)?;

    Ok(global_index)
}

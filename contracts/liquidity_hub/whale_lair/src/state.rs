use cosmwasm_std::{Addr, Decimal, DepsMut, StdError, StdResult, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use white_whale::whale_lair::{AssetInfo, Bond, Config, GlobalIndex};

use crate::ContractError;

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

    bond.weight = get_weight(
        timestamp,
        bond.weight,
        bond.asset.amount,
        config.growth_rate,
        bond.timestamp,
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

    global_index.weight = get_weight(
        timestamp,
        global_index.weight,
        global_index.bond_amount,
        config.growth_rate,
        global_index.timestamp,
    )?;

    global_index.timestamp = timestamp;

    GLOBAL.save(deps.storage, &global_index)?;

    Ok(global_index)
}

/// Calculates the bonding weight of the given amount for the provided timestamps.
pub fn get_weight(
    current_timestamp: Timestamp,
    weight: Uint128,
    amount: Uint128,
    growth_rate: Decimal,
    timestamp: Timestamp,
) -> StdResult<Uint128> {
    let time_factor = Uint128::from(
        Timestamp::from_nanos(
            current_timestamp
                .seconds()
                .checked_sub(timestamp.seconds())
                .ok_or_else(|| StdError::generic_err("Error calculating time_factor"))?,
        )
        .nanos(),
    );

    // convert Uint128 to decimal to do the operation  weight = weight + amount * (current_timestamp - timestamp) * growth_rate
    //let amount = Decimal256::from_ratio(amount, Uint128::one());
    //let time_factor = Decimal256::from_ratio(time_factor, Uint128::one());

    Ok(weight.checked_add(amount.checked_mul(time_factor)? * growth_rate)?)
}

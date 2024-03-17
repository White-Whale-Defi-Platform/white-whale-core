#![cfg(not(tarpaulin_include))]
use crate::state::CONFIG;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, DepsMut, StdError, Uint64};
use cw_storage_plus::Item;
use white_whale_std::pool_network::asset::AssetInfo;
use white_whale_std::whale_lair::Config;

pub fn migrate(deps: DepsMut) -> Result<(), StdError> {
    Ok(())
}

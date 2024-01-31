#![cfg(not(tarpaulin_include))]

use classic_bindings::TerraQuery;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, StdError};
use cw_storage_plus::Item;
use white_whale_std::fee_collector::Config;

use crate::state::CONFIG;

/// Migrates state from v1.0.5 and lower to v1.1.0, which includes different contract addresses
/// in the Config.
pub fn migrate_to_v110(deps: DepsMut<TerraQuery>) -> Result<(), StdError> {
    #[cw_serde]
    struct ConfigV105 {
        pub owner: Addr,
    }

    #[cw_serde]
    struct ConfigV110 {
        pub owner: Addr,
        pub pool_router: Addr,
        pub fee_distributor: Addr,
        pub pool_factory: Addr,
        pub vault_factory: Addr,
    }

    const CONFIGV105: Item<ConfigV105> = Item::new("config");
    let config_v105 = CONFIGV105.load(deps.storage)?;

    let config = Config {
        owner: config_v105.owner,
        pool_router: Addr::unchecked(""),
        fee_distributor: Addr::unchecked(""),
        pool_factory: Addr::unchecked(""),
        vault_factory: Addr::unchecked(""),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(())
}

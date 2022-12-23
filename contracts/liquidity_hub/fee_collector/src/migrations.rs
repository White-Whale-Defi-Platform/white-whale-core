#![cfg(not(tarpaulin_include))]
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, StdError};
use cw_storage_plus::Item;

use crate::state::{Config, CONFIG};

/// Migrates state from v1.0.5 and lower to v1.1.0, which includes the pool router address in the Config.
pub fn migrate_to_v110(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    struct ConfigV105 {
        pub owner: Addr,
    }

    const CONFIGV105: Item<ConfigV105> = Item::new("config");
    let config_v105 = CONFIGV105.load(deps.storage)?;

    let config = Config {
        owner: config_v105.owner,
        pool_router: Addr::unchecked(""),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(())
}

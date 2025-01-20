#![cfg(not(tarpaulin_include))]
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, DepsMut, StdError};
use cw_storage_plus::Item;
use white_whale_std::fee_collector::Config;

use crate::state::CONFIG;

/// Migrates state from pre v1.2.0, which includes the take rate, the take rate dao address and the
/// feature flag for the take rate
pub fn migrate_to_v120(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    struct ConfigPreV120 {
        pub owner: Addr,
        pub pool_router: Addr,
        pub fee_distributor: Addr,
        pub pool_factory: Addr,
        pub vault_factory: Addr,
    }

    const CONFIGPREV120: Item<ConfigPreV120> = Item::new("config");
    let config_pre_v120 = CONFIGPREV120.load(deps.storage)?;

    let config = Config {
        owner: config_pre_v120.owner,
        pool_router: config_pre_v120.pool_router,
        fee_distributor: config_pre_v120.fee_distributor,
        pool_factory: config_pre_v120.pool_factory,
        vault_factory: config_pre_v120.vault_factory,
        take_rate: Decimal::zero(),
        take_rate_dao_address: Addr::unchecked(""),
        is_take_rate_active: false,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(())
}

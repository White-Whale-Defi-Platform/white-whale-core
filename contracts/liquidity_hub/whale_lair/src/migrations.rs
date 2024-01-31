#![cfg(not(tarpaulin_include))]
use crate::state::CONFIG;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, DepsMut, StdError, Uint64};
use cw_storage_plus::Item;
use white_whale_std::pool_network::asset::AssetInfo;
use white_whale_std::whale_lair::Config;

pub fn migrate_to_v090(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    pub struct ConfigV080 {
        /// Owner of the contract.
        pub owner: Addr,
        /// Unbonding period in nanoseconds.
        pub unbonding_period: Uint64,
        /// A fraction that controls the effect of time on the weight of a bond. If the growth rate is set
        /// to zero, time will have no impact on the weight.
        pub growth_rate: Decimal,
        /// Denom of the asset to be bonded. Can't only be set at instantiation.
        pub bonding_assets: Vec<AssetInfo>,
    }
    const CONFIGV080: Item<ConfigV080> = Item::new("config");
    let config_v080 = CONFIGV080.load(deps.storage)?;

    let config = Config {
        owner: config_v080.owner,
        unbonding_period: config_v080.unbonding_period,
        growth_rate: config_v080.growth_rate,
        bonding_assets: config_v080.bonding_assets,
        fee_distributor_addr: Addr::unchecked(""), // set it empty, then update with the new value
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(())
}

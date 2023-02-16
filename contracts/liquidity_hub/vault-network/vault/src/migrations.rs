#![cfg(not(tarpaulin_include))]
use crate::state::{initialize_fee, ALL_TIME_BURNED_FEES, CONFIG};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, DepsMut, StdError};
use cw_storage_plus::Item;
use pool_network::asset::AssetInfo;
use vault_network::vault::Config;
use white_whale::fee::{Fee, VaultFee};

pub fn migrate_to_v120(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    pub struct ConfigV113 {
        /// The owner of the vault
        pub owner: Addr,
        /// The asset info the vault manages
        pub asset_info: AssetInfo,
        /// If flash-loans are enabled
        pub flash_loan_enabled: bool,
        /// If deposits are enabled
        pub deposit_enabled: bool,
        /// If withdrawals are enabled
        pub withdraw_enabled: bool,
        /// The address of the liquidity token
        pub liquidity_token: Addr,
        /// The address of the fee collector
        pub fee_collector_addr: Addr,
        /// The fees associated with this vault
        pub fees: VaultFeeV113,
    }

    #[cw_serde]
    pub struct VaultFeeV113 {
        pub protocol_fee: Fee,
        pub flash_loan_fee: Fee,
    }

    pub const CONFIG_V113: Item<ConfigV113> = Item::new("config");
    let config_v113 = CONFIG_V113.load(deps.storage)?;

    // Add burn fee to config. Zero fee is used as default.
    let config = Config {
        owner: config_v113.owner,
        asset_info: config_v113.asset_info,
        flash_loan_enabled: config_v113.flash_loan_enabled,
        deposit_enabled: config_v113.deposit_enabled,
        withdraw_enabled: config_v113.withdraw_enabled,
        liquidity_token: config_v113.liquidity_token,
        fee_collector_addr: config_v113.fee_collector_addr,
        fees: VaultFee {
            protocol_fee: config_v113.fees.protocol_fee,
            flash_loan_fee: config_v113.fees.flash_loan_fee,
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        },
    };

    CONFIG.save(deps.storage, &config)?;

    // initialize the burned fee storage item
    initialize_fee(deps.storage, ALL_TIME_BURNED_FEES, config.asset_info)?;

    Ok(())
}

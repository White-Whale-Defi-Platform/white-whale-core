use cosmwasm_std::{Coin, DepsMut, MessageInfo, Response};
use white_whale_std::pool_manager::{Config, FeatureToggle};

use crate::{state::CONFIG, ContractError};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    bonding_manager_addr: Option<String>,
    pool_creation_fee: Option<Coin>,
    feature_toggle: Option<FeatureToggle>,
) -> Result<Response, ContractError> {
    // permission check
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    CONFIG.update(deps.storage, |mut config| {
        if let Some(new_bonding_manager_addr) = bonding_manager_addr {
            let bonding_manager_addr = deps.api.addr_validate(&new_bonding_manager_addr)?;
            config.bonding_manager_addr = bonding_manager_addr;
        }

        if let Some(pool_creation_fee) = pool_creation_fee {
            config.pool_creation_fee = pool_creation_fee;
        }

        if let Some(feature_toggle) = feature_toggle {
            config.feature_toggle = feature_toggle;
        }
        Ok::<Config, ContractError>(config)
    })?;

    Ok(Response::default().add_attribute("action", "update_config"))
}

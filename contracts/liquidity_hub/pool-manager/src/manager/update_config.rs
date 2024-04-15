use cosmwasm_std::{ensure, Coin, DepsMut, MessageInfo, Response};
use white_whale_std::pool_manager::Config;
use white_whale_std::pool_network::pair::FeatureToggle;

use crate::{state::MANAGER_CONFIG, ContractError};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    whale_lair_addr: Option<String>,
    pool_creation_fee: Option<Coin>,
    feature_toggle: Option<FeatureToggle>,
) -> Result<Response, ContractError> {
    // permission check
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    MANAGER_CONFIG.update(deps.storage, |mut config| {
        if let Some(whale_lair_addr) = whale_lair_addr {
            let whale_lair_addr = deps.api.addr_validate(&whale_lair_addr)?;
            config.whale_lair_addr = whale_lair_addr;
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

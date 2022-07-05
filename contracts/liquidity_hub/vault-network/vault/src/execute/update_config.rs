use cosmwasm_std::{DepsMut, Response, StdError, StdResult};

use crate::state::CONFIG;

pub fn update_config(
    deps: DepsMut,
    flash_loan_enabled: Option<bool>,
    withdraw_enabled: Option<bool>,
    deposit_enabled: Option<bool>,
    new_owner: Option<String>,
) -> StdResult<Response> {
    let config = CONFIG.update::<_, StdError>(deps.storage, |mut config| {
        // if user leaves as None, do not perform change operation
        if let Some(flash_loan_enabled) = flash_loan_enabled {
            config.flash_loan_enabled = flash_loan_enabled;
        }
        if let Some(withdraw_enabled) = withdraw_enabled {
            config.withdraw_enabled = withdraw_enabled;
        }
        if let Some(deposit_enabled) = deposit_enabled {
            config.deposit_enabled = deposit_enabled;
        }
        if let Some(new_owner) = new_owner {
            config.owner = deps.api.addr_validate(&new_owner)?;
        }

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![
        ("method", "update_config"),
        ("flash_loan_enabled", &config.flash_loan_enabled.to_string()),
        ("withdraw_enabled", &config.withdraw_enabled.to_string()),
        ("deposit_enabled", &config.deposit_enabled.to_string()),
        ("owner", &config.owner.into_string()),
    ]))
}

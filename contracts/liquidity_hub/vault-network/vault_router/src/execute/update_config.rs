use cosmwasm_std::{DepsMut, MessageInfo, Response};

use crate::{
    err::{StdResult, VaultRouterError},
    state::CONFIG,
};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
    new_vault_factory_addr: Option<String>,
) -> StdResult<Response> {
    let new_config = CONFIG.update::<_, VaultRouterError>(deps.storage, |mut config| {
        // check that sender is the owner
        if info.sender != config.owner {
            return Err(VaultRouterError::Unauthorized {});
        }

        if let Some(new_owner) = new_owner {
            config.owner = deps.api.addr_validate(&new_owner)?;
        };

        if let Some(new_vault_factory_addr) = new_vault_factory_addr {
            config.vault_factory = deps.api.addr_validate(&new_vault_factory_addr)?;
        }

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![
        ("method", "update_config"),
        ("owner", &new_config.owner.into_string()),
        ("vault_factory", &new_config.vault_factory.into_string()),
    ]))
}

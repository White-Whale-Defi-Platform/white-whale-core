use cosmwasm_std::{DepsMut, MessageInfo, Response};

use crate::{
    err::{StdResult, VaultFactoryError},
    state::CONFIG,
};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: Option<String>,
) -> StdResult<Response> {
    let new_config = CONFIG.update::<_, VaultFactoryError>(deps.storage, |mut config| {
        // check that sender is the owner
        if info.sender != config.owner {
            return Err(VaultFactoryError::Unauthorized {});
        }

        if let Some(new_owner) = new_owner {
            config.owner = deps.api.addr_validate(&new_owner)?;
        };

        Ok(config)
    })?;

    Ok(Response::new().add_attributes(vec![
        ("method", "update_config"),
        ("owner", &new_config.owner.into_string()),
    ]))
}

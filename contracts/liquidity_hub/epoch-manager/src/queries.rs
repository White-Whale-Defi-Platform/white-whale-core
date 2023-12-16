use cosmwasm_std::{Addr, Deps, Order, StdError, StdResult};
use cw_controllers::HooksResponse;

use white_whale::epoch_manager::epoch_manager::{ConfigResponse, EpochResponse};

use crate::state::{ADMIN, CONFIG, EPOCHS, HOOKS};

/// Queries the config. Returns a [ConfigResponse].
pub(crate) fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let admin = ADMIN.get(deps)?.unwrap_or(Addr::unchecked(""));
    let config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: admin,
        epoch_config: config.epoch_config,
    })
}

/// Queries the current epoch. Returns an [EpochResponse].
pub(crate) fn query_current_epoch(deps: Deps) -> StdResult<EpochResponse> {
    let option = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .next();

    let epoch = match option {
        Some(Ok((_, epoch))) => epoch,
        _ => Err(StdError::generic_err("No epochs stored"))?,
    };

    Ok(EpochResponse { epoch })
}

/// Queries the current epoch. Returns an [EpochResponse].
pub(crate) fn query_epoch(deps: Deps, id: u64) -> StdResult<EpochResponse> {
    let epoch = EPOCHS
        .may_load(deps.storage, &id.to_be_bytes())?
        .ok_or_else(|| StdError::generic_err(format!("No epoch found with id {}", id)))?;
    Ok(epoch.to_epoch_response())
}

/// Queries hooks. Returns a [HooksResponse].
pub(crate) fn query_hooks(deps: Deps) -> StdResult<HooksResponse> {
    HOOKS.query_hooks(deps)
}

/// Check whether or not a hook is in the registry. Returns a [bool].
pub(crate) fn query_hook(deps: Deps, hook: String) -> StdResult<bool> {
    HOOKS.query_hook(deps, hook)
}

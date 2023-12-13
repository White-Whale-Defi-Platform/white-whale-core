use cosmwasm_std::{Addr, Deps, StdResult};
use cw_controllers::HooksResponse;

use white_whale::epoch_manager::epoch_manager::{ConfigResponse, Epoch, EpochResponse};

use crate::state::{ADMIN, CONFIG, EPOCH, HOOKS};

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
    EPOCH
        .load(deps.storage)
        .map(|epoch| epoch.to_epoch_response())
}

/// Queries the current epoch. Returns an [EpochResponse].
pub(crate) fn query_epoch(deps: Deps, id: u64) -> StdResult<EpochResponse> {
    let current_epoch = EPOCH.load(deps.storage)?;

    if current_epoch.id == id {
        Ok(current_epoch.to_epoch_response())
    } else {
        let epoch_difference = current_epoch.id.saturating_sub(id);

        let epoch = Epoch {
            id,
            start_time: current_epoch.start_time.minus_nanos(
                CONFIG.load(deps.storage)?.epoch_config.duration.u64() * epoch_difference,
            ),
        };
        Ok(epoch.to_epoch_response())
    }
}

/// Queries hooks. Returns a [HooksResponse].
pub(crate) fn query_hooks(deps: Deps) -> StdResult<HooksResponse> {
    HOOKS.query_hooks(deps)
}

/// Check whether or not a hook is in the registry. Returns a [bool].
pub(crate) fn query_hook(deps: Deps, hook: String) -> StdResult<bool> {
    HOOKS.query_hook(deps, hook)
}

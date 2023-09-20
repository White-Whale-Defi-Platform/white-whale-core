use std::collections::BTreeMap;

// currently a stub file
// until migrations are needed in the future
use cosmwasm_std::{DepsMut, StdError};

use crate::queries::get_flows;
use crate::state::FLOWS;

/// Migrates to version 1.0.5, which introduces the [Flow] field asset_history.
pub(crate) fn migrate_to_v105(deps: DepsMut) -> Result<(), StdError> {
    let mut flows = get_flows(deps.as_ref(), None, None)?;

    // add the asset_history field to all available flows
    for flow in flows.iter_mut() {
        flow.asset_history = BTreeMap::new();
        flow.flow_label = None;

        FLOWS.save(deps.storage, (flow.start_epoch, flow.flow_id), flow)?;
    }

    Ok(())
}

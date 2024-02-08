#![cfg(not(tarpaulin_include))]

use std::collections::{BTreeMap, HashMap};

use cosmwasm_schema::cw_serde;
// currently a stub file
// until migrations are needed in the future
use cosmwasm_std::{Addr, DepsMut, Order, StdError, StdResult, Uint128};
use cw_storage_plus::Map;

use white_whale_std::pool_network::asset::Asset;
use white_whale_std::pool_network::incentive::{Curve, Flow};

use crate::state::{EpochId, FlowId, FLOWS};

/// Migrates to version 1.0.6, which introduces the [Flow] field asset_history.
pub(crate) fn migrate_to_v106(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    pub struct FlowV104 {
        /// A unique identifier of the flow.
        pub flow_id: u64,
        /// The account which opened the flow and can manage it.
        pub flow_creator: Addr,
        /// The asset the flow was created to distribute.
        pub flow_asset: Asset,
        /// The amount of the `flow_asset` that has been claimed so far.
        pub claimed_amount: Uint128,
        /// The type of curve the flow has.
        pub curve: Curve, //todo not doing anything for now
        /// The epoch at which the flow starts.
        pub start_epoch: u64,
        /// The epoch at which the flow ends.
        pub end_epoch: u64,
        /// emitted tokens
        pub emitted_tokens: HashMap<u64, Uint128>,
    }

    // load old flows map
    pub const FLOWS_V104: Map<(EpochId, FlowId), FlowV104> = Map::new("flows");

    let flows = FLOWS_V104
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<(_, FlowV104)>>>()?
        .into_iter()
        .map(|(_, flow)| flow)
        .collect::<Vec<FlowV104>>();

    // add the asset_history field to all available flows
    for f in flows.iter() {
        let flow = Flow {
            flow_id: f.clone().flow_id,
            flow_label: None, //new field
            flow_creator: f.clone().flow_creator,
            flow_asset: f.clone().flow_asset,
            claimed_amount: f.clone().claimed_amount,
            curve: f.clone().curve,
            start_epoch: f.clone().start_epoch,
            end_epoch: f.clone().end_epoch,
            emitted_tokens: f.clone().emitted_tokens,
            asset_history: BTreeMap::new(), //new field
        };

        FLOWS.save(deps.storage, (f.start_epoch, f.flow_id), &flow)?;
    }

    Ok(())
}

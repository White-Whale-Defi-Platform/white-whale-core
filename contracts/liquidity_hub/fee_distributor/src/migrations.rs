#![cfg(not(tarpaulin_include))]
use crate::state::{CONFIG, EPOCHS};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, DepsMut, Order, QueryRequest, StdError, StdResult, Timestamp, Uint64, WasmQuery,
};
use cw_storage_plus::Map;
use white_whale::fee_distributor::Epoch;
use white_whale::pool_network::asset::Asset;
use white_whale::whale_lair::GlobalIndex;
use white_whale::whale_lair::QueryMsg as LairQueryMsg;

/// Migrates state from the first iteration, v0.8.* to v0.9.0, which includes the global index in
/// the Epoch. This was done to fix bonding issues.
pub fn migrate_to_v090(deps: DepsMut) -> Result<(), StdError> {
    #[cw_serde]
    #[derive(Default)]
    pub struct EpochV08 {
        // Epoch identifier
        pub id: Uint64,
        // Epoch start time
        pub start_time: Timestamp,
        // Initial fees to be distributed in this epoch.
        pub total: Vec<Asset>,
        // Fees left to be claimed on this epoch. These available fees are forwarded when the epoch expires.
        pub available: Vec<Asset>,
        // Fees that were claimed on this epoch. For keeping record on the total fees claimed.
        pub claimed: Vec<Asset>,
    }

    const EPOCHSV08: Map<&[u8], Epoch> = Map::new("epochs");

    let epochs_v08 = EPOCHSV08
        .range(deps.storage, None, None, Order::Descending)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<Epoch>>>()?;

    let bonding_contract_addr = CONFIG.load(deps.storage)?.bonding_contract_addr;
    // Query the current global index
    let global_index: GlobalIndex = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: bonding_contract_addr.to_string(),
        msg: to_binary(&LairQueryMsg::GlobalIndex {})?,
    }))?;

    for mut epoch in epochs_v08 {
        // assign the current global index to all epochs
        epoch = Epoch {
            global_index: global_index.clone(),
            ..epoch
        };

        EPOCHS.save(deps.storage, &epoch.id.to_be_bytes(), &epoch)?;
    }

    Ok(())
}

#![cfg(not(tarpaulin_include))]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Order, QueryRequest, StdError, StdResult, Timestamp, Uint64,
    WasmQuery,
};
use cw_storage_plus::Map;

use white_whale::fee_distributor::Epoch;
use white_whale::pool_network::asset;
use white_whale::pool_network::asset::Asset;
use white_whale::whale_lair::GlobalIndex;
use white_whale::whale_lair::QueryMsg as LairQueryMsg;

use crate::state::{get_claimable_epochs, CONFIG, EPOCHS};

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

    const EPOCHSV08: Map<&[u8], EpochV08> = Map::new("epochs");

    let epochs_v08 = EPOCHSV08
        .range(deps.storage, None, None, Order::Descending)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<EpochV08>>>()?;

    let bonding_contract_addr = CONFIG.load(deps.storage)?.bonding_contract_addr;
    // Query the current global index
    let global_index: GlobalIndex = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: bonding_contract_addr.to_string(),
        msg: to_binary(&LairQueryMsg::GlobalIndex {})?,
    }))?;

    for epoch in epochs_v08 {
        // assign the current global index to all epochs
        let epochv090 = Epoch {
            id: epoch.id,
            start_time: epoch.start_time,
            total: epoch.total,
            available: epoch.available,
            claimed: epoch.claimed,
            global_index: global_index.clone(),
        };

        EPOCHS.save(deps.storage, &epochv090.id.to_be_bytes(), &epochv090)?;
    }

    Ok(())
}

/// Fixes the broken state for Epochs created prior to the v0.9.0 migration.
pub fn migrate_to_v091(deps: DepsMut) -> Result<Vec<CosmosMsg>, StdError> {
    let claimable_epochs = get_claimable_epochs(deps.as_ref())?;

    // 14 June 2023 16:00:00 UTC
    let migration_timestamp = Timestamp::from_seconds(1686758400);
    let mut faulty_epochs = claimable_epochs
        .epochs
        .into_iter()
        .filter(|epoch| epoch.start_time.seconds() < migration_timestamp.seconds())
        .collect::<Vec<Epoch>>();

    let fee_collector_addr = CONFIG.load(deps.storage)?.fee_collector_addr;

    // collect all available funds on faulty epochs and send them back to the fee collector, to be
    // redistributed on the next (new) epoch

    let mut total_fees: Vec<Asset> = vec![];
    for epoch in faulty_epochs.iter_mut() {
        total_fees = asset::aggregate_assets(total_fees, epoch.available.clone())?;

        // set the available fees of this faulty epoch to zero
        epoch.available = vec![];

        // save the faulty epoch in the state
        EPOCHS.save(deps.storage, &epoch.id.to_be_bytes(), epoch)?;
    }

    // create messages to send total_fees back to the fee collector
    let mut messages = vec![];

    for fee in total_fees {
        messages.push(fee.into_msg(fee_collector_addr.clone())?);
    }

    Ok(messages)
}

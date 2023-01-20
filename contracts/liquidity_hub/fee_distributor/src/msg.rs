use cosmwasm_schema::{cw_serde, QueryResponses};

use terraswap::asset::Asset;

use crate::state::{Config, Epoch};

#[cw_serde]
pub struct InstantiateMsg {
    pub staking_contract_addr: String,
    pub fee_collector_addr: String,
    pub grace_period: u128,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a new epoch, forwarding available tokens from epochs that are past the grace period.
    /// Can only be executed by the fee collector.
    NewEpoch { fees: Vec<Asset> },

    /// Claims tokens from the current epoch and all epochs that are in the grace period.
    /// Sends all tokens to the sender.
    Claim {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(Config)]
    Config {},

    /// Returns the current epoch, which is the last on the EPOCHS map.
    #[returns(Epoch)]
    CurrentEpoch {},

    /// Returns the epoch with the given id.
    #[returns(Option<Epoch>)]
    Epoch { id: u128 },

    /// Returns the [Epoch]s that can be claimed.
    #[returns(Vec<Epoch>)]
    ClaimableEpochs {},
}

use cosmwasm_std::Uint128;

pub mod commands;

/// Minimum amount of an asset to create an incentive with
pub(crate) const MIN_INCENTIVE_AMOUNT: Uint128 = Uint128::new(1_000u128);
// If the end_epoch is not specified, the incentive will be expanded by DEFAULT_INCENTIVE_DURATION when
// the current epoch is within INCENTIVE_EXPANSION_BUFFER epochs from the end_epoch.
pub(crate) const INCENTIVE_EXPANSION_BUFFER: u64 = 5u64;
// An incentive can only be expanded for a maximum of INCENTIVE_EXPANSION_LIMIT epochs. If that limit is exceeded,
// the flow is "reset", shifting the start_epoch to the current epoch and the end_epoch to the current_epoch + DEFAULT_FLOW_DURATION.
// Unclaimed assets become the flow.asset and both the flow.asset_history and flow.emitted_tokens is cleared.
pub(crate) const INCENTIVE_EXPANSION_LIMIT: u64 = 180u64;

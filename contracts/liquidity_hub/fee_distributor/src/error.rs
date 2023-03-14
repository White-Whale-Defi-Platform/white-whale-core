use cosmwasm_std::{DivideByZeroError, OverflowError, StdError, Uint64};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid grace period: {0}. Must be between 1 and 10.")]
    InvalidGracePeriod(Uint64),

    #[error("Invalid epoch duration: {0}. Must be at least 1 day.")]
    InvalidEpochDuration(Uint64),

    #[error("Invalid epoch start hour: {0}. Must be between 0 and 23. Example, 0 is 12am, 12 is 12pm, 23 is 11pm.")]
    InvalidEpochStartHour(Uint64),

    #[error("The assets sent don't match the assets expected.")]
    AssetMismatch {},

    #[error("There are no claimable fees.")]
    NothingToClaim {},

    #[error("The rewards cannot exceed the available claimable fees.")]
    InvalidReward {},

    #[error("The current epoch epoch has not expired yet.")]
    CurrentEpochNotExpired {},

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),
}

use cosmwasm_std::{DivideByZeroError, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid grace period: {0}. Must be between 1 and 10.")]
    InvalidGracePeriod(u128),

    #[error("The assets sent don't match the assets expected.")]
    AssetMismatch {},

    #[error("There are no claimable fees.")]
    NothingToClaim {},

    #[error("The rewards cannot exceed the available claimable fees.")]
    InvalidReward {},

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),
}

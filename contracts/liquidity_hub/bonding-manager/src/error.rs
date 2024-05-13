use cosmwasm_std::{DivideByZeroError, OverflowError, StdError, Uint128};
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("The asset sent doesn't match the asset expected. Please check the denom and amount.")]
    AssetMismatch,

    #[error("The amount of tokens to unbond is greater than the amount of tokens bonded.")]
    InsufficientBond,

    #[error("The amount of tokens to unbond must be greater than zero.")]
    InvalidUnbondingAmount,

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("The growth rate must be between 0 and 1. i.e. 0.5 for 50%")]
    InvalidGrowthRate,

    #[error(
        "The amount of bonding assets is greater than the limit allowed. Limit is {0}, sent {1}."
    )]
    InvalidBondingAssetsLimit(usize, usize),

    #[error("Nothing to unbond")]
    NothingToUnbond,

    #[error("Nothing to withdraw")]
    NothingToWithdraw,

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("There are unclaimed rewards available. Claim them before attempting to bond/unbond")]
    UnclaimedRewards,

    #[error("Trying to bond before an epoch has been created")]
    EpochNotCreatedYet,

    #[error("Nothing to claim")]
    NothingToClaim,

    #[error("Something is off with the reward calculation, user share is above 1. Can't claim.")]
    InvalidShare,

    #[error(
        "Invalid reward amount. Reward: {reward}, but only {available} available in the reward bucket."
    )]
    InvalidReward { reward: Uint128, available: Uint128 },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}

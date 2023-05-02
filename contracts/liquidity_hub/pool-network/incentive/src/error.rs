use cosmwasm_std::{
    CheckedFromRatioError, ConversionOverflowError, DivideByZeroError, OverflowError, StdError,
    Uint128,
};
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Attempt to create a new incentive flow, which exceeds the maximum of {maximum} flows allowed")]
    TooManyFlows {
        /// The maximum amount of liquidity flows that can exist
        maximum: u64,
    },

    #[error("Specified flow asset was not transferred to incentive contract")]
    FlowAssetNotSent,

    #[error("Flow end timestamp was set to a time in the past")]
    FlowExpirationInPast,

    #[error("Flow start timestamp is too far into the future")]
    FlowStartTooFar,

    #[error("Flow identifier ({invalid_id}) does not point to any flow")]
    NonExistentFlow { invalid_id: u64 },

    #[error("Account not permitted to close flow {flow_id}")]
    UnauthorizedFlowClose { flow_id: u64 },

    #[error("Flow creation fee was not included")]
    FlowFeeMissing,

    #[error("Flow creation fee was not fulfilled, only {paid_amount} / {required_amount} present")]
    FlowFeeNotPaid {
        /// The amount that was paid
        paid_amount: Uint128,
        /// The amount that needed to be paid
        required_amount: Uint128,
    },

    #[error("Attempt to create a position with {deposited_amount}, but only {allowance_amount} was set in allowance")]
    MissingPositionDeposit {
        /// The actual amount that the contract has an allowance for.
        allowance_amount: Uint128,
        /// The amount the account attempted to open a position with
        deposited_amount: Uint128,
    },

    #[error(
        "Attempt to create a new position with the same unbonding duration as an existing position"
    )]
    DuplicatePosition,

    #[error("Unbonding timestamp overflowed")]
    OverflowTimestamp,

    #[error(
        "Invalid unbonding duration of {specified} specified, must be between {min} and {max}"
    )]
    InvalidUnbondingDuration {
        /// The minimum amount of seconds that a user must bond for.
        min: u64,
        /// The maximum amount of seconds that a user can bond for.
        max: u64,
        /// The amount of seconds the user attempted to bond for.
        specified: u64,
    },

    #[error("Overflowed when calculating the weight to give to user")]
    WeightOverflowCalculation,

    #[error("Failed to find position with unbonding_duration of {unbonding_duration}")]
    NonExistentPosition {
        /// The unbonding duration the account expected to find a position with.
        unbonding_duration: u64,
    },

    #[error("Attempt to expand position which has already completed its unbonding")]
    ExpiredPosition,

    #[error("Attempt to compute the weight of a duration of {unbonding_duration} which is outside the allowed bounds")]
    InvalidWeight { unbonding_duration: u64 },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}

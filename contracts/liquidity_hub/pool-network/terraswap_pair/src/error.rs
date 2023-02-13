use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, ConversionOverflowError, DivideByZeroError,
    OverflowError, StdError, Uint128,
};
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error(transparent)]
    CheckedMultiplyRatioError(#[from] CheckedMultiplyRatioError),

    #[error(transparent)]
    CheckedFromRatioError(#[from] CheckedFromRatioError),

    #[error(transparent)]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error(transparent)]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Spread limit exceeded")]
    MaxSpreadAssertion {},

    #[error("Slippage tolerance exceeded")]
    MaxSlippageAssertion {},

    #[error("The asset doesn't match the assets stored in contract")]
    AssetMismatch {},

    #[error("Too small offer amount")]
    TooSmallOfferAmount {},

    #[error("Operation disabled, {0}")]
    OperationDisabled(String),

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Initial liquidity amount must be over {0}")]
    InvalidInitialLiquidityAmount(Uint128),

    #[error("Failed to compute the LP share with the given deposit")]
    LiquidityShareComputation {},

    #[error("Failed to converge when performing newtons method")]
    ConvergeError {},

    #[error("An conversion overflow occurred when attempting to swap an asset")]
    SwapOverflowError {},

    #[error("An overflow occurred when attempting to construct a decimal")]
    DecimalOverflow {},
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}

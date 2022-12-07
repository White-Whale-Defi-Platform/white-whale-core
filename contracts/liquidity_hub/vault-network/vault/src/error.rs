use cosmwasm_std::{ConversionOverflowError, DivideByZeroError, OverflowError, StdError, Uint128};
use semver::Version;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum VaultError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    ConversionOverflowError(#[from] ConversionOverflowError),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("Deposits are not enabled")]
    DepositsDisabled {},

    #[error("Flash-loans are not enabled")]
    FlashLoansDisabled {},

    #[error("mismatch of sent {sent} but specified deposit amount of {wanted}")]
    FundsMismatch { sent: Uint128, wanted: Uint128 },

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Attempt to call callback function outside contract")]
    ExternalCallback {},

    #[error(
        "Final desired amount of {required_amount} is less than current balance of {current_balance} (got {old_balance} -> {current_balance}, want {old_balance} -> {required_amount})"
    )]
    NegativeProfit {
        /// The balance before the loan occurred
        old_balance: Uint128,
        /// The current balance of the vault
        current_balance: Uint128,
        /// The required return amount for the vault
        required_amount: Uint128,
    },

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Withdrawals are not enabled")]
    WithdrawsDisabled {},

    #[error("Cannot deposit while flash-loaning")]
    DepositDuringLoan {},
}

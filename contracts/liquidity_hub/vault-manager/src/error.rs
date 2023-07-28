use cosmwasm_std::{DivideByZeroError, OverflowError, StdError, Uint128};
use cw_utils::PaymentError;
use semver::Version;
use thiserror::Error;

use white_whale::pool_network::asset::AssetInfo;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("mismatch of sent {sent} but specified deposit amount of {wanted}")]
    FundsMismatch { sent: Uint128, wanted: Uint128 },

    #[error("The asset \"{asset_info}\" already has a vault")]
    ExistingVault { asset_info: AssetInfo },

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error("Vault doesn't exist")]
    NonExistentVault {},

    #[error("Invalid vault creation fee paid. Received {amount}, expected {expected}")]
    InvalidVaultCreationFee { amount: Uint128, expected: Uint128 },

    #[error("The token factory feature is not enabled")]
    TokenFactoryNotEnabled {},

    #[error("Invalid LpTokenType")]
    InvalidLpTokenType {},

    #[error("Initial liquidity amount must be over {0}")]
    InvalidInitialLiquidityAmount(Uint128),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("{0}")]
    DivideByZeroError(#[from] DivideByZeroError),

    #[error("{0}")]
    PaymentError(#[from] PaymentError),
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}

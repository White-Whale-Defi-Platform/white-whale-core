use crate::liquidity::commands::MAX_ASSETS_PER_POOL;
use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, ConversionOverflowError, DivideByZeroError,
    Instantiate2AddressError, OverflowError, StdError, Uint128,
};
use cw_ownable::OwnershipError;
use cw_utils::PaymentError;
use semver::Version;
use thiserror::Error;


#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    // Handle all normal errors from the StdError
    #[error("{0}")]
    Std(#[from] StdError),

    // Handle errors specific to payments from cw-util
    #[error("{0}")]
    PaymentError(#[from] PaymentError),

    #[error(transparent)]
    Instantiate2Error(#[from] Instantiate2AddressError),

    // Handle ownership errors from cw-ownable
    #[error("{0}")]
    OwnershipError(#[from] OwnershipError),

    // Handle Upgrade/Migrate related semver errors
    #[error("Semver parsing error: {0}")]
    SemVer(String),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("The provided assets are both the same")]
    SameAsset {},

    #[error("Invalid operations; multiple output token")]
    MultipleOutputToken {},

    #[error("Attempt to migrate to version {new_version}, but contract is on a higher version {current_version}")]
    MigrateInvalidVersion {
        new_version: Version,
        current_version: Version,
    },

    #[error(
        "Assertion failed; minimum receive amount: {minimum_receive}, swap amount: {swap_amount}"
    )]
    MinimumReceiveAssertion {
        minimum_receive: Uint128,
        swap_amount: Uint128,
    },

    #[error(
        "The asset \"{asset_infos}\" with the identifier \"{identifier}\" already has a vault"
    )]
    PairExists {
        asset_infos: String, //String representation of the asset infos
        identifier: String,
    },

    #[error("More assets provided than is supported the max is currently {MAX_ASSETS_PER_POOL}, you provided {assets_provided}")]
    TooManyAssets { assets_provided: usize },

    #[error("{asset} is invalid")]
    InvalidAsset { asset: String },

    #[error("Pair already exist")]
    ExistingPair {},

    #[error("Operation disabled, {0}")]
    OperationDisabled(String),

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Initial liquidity amount must be over {0}")]
    InvalidInitialLiquidityAmount(Uint128),

    #[error("Failed to compute the LP share with the given deposit")]
    LiquidityShareComputation {},

    #[error("Spread limit exceeded")]
    MaxSpreadAssertion {},

    #[error("Slippage tolerance exceeded")]
    MaxSlippageAssertion {},

    #[error("The asset doesn't match the assets stored in contract")]
    AssetMismatch {},

    #[error("Too small offer amount")]
    TooSmallOfferAmount {},

    #[error("Failed to converge when performing newtons method")]
    ConvergeError {},

    #[error("An conversion overflow occurred when attempting to swap an asset")]
    SwapOverflowError {},

    #[error("An overflow occurred when attempting to construct a decimal")]
    DecimalOverflow {},

    #[error("A balance greater than zero is required by the factory to verify the asset")]
    InvalidVerificationBalance {},

    #[error("Burn fee is not allowed when using factory tokens")]
    TokenFactoryAssetBurnDisabled {},

    #[error("The token factory feature is not enabled")]
    TokenFactoryNotEnabled {},

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

    #[error("No swap route found for {offer_asset} -> {ask_asset}")]
    NoSwapRouteForAssets {
        offer_asset: String,
        ask_asset: String,
    },

    #[error("Must provide swap operations to execute")]
    NoSwapOperationsProvided {},
    #[error("Invalid pair creation fee, expected {expected} got {amount}")]
    InvalidPairCreationFee {
        amount: cosmwasm_std::Uint128,
        expected: cosmwasm_std::Uint128,
    },
}

impl From<semver::Error> for ContractError {
    fn from(err: semver::Error) -> Self {
        Self::SemVer(err.to_string())
    }
}

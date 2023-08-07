use crate::commands::MAX_ASSETS_PER_POOL;
use cosmwasm_std::{
    CheckedFromRatioError, CheckedMultiplyRatioError, ConversionOverflowError, DivideByZeroError,
    OverflowError, StdError, Uint128,
};use thiserror::Error;
#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("The provided assets are both the same")]
    SameAsset {},

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
}

mod feature_toggle;
mod protocol_fees;
mod provide_liquidity;
mod queries;
mod stableswap;
mod swap;
mod testing;
mod withdrawals;

#[cfg(feature = "injective")]
mod mock_app;
#[cfg(feature = "injective")]
mod mock_instantiate;
#[cfg(feature = "injective")]
mod store_code;

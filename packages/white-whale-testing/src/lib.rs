#[cfg(not(target_arch = "wasm32"))]
pub mod integration;

#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
pub mod multi_test;

pub mod asset;
#[cfg(feature = "injective")]
pub mod denom_injective;
#[cfg(feature = "osmosis_token_factory")]
pub mod denom_osmosis;
pub mod factory;
pub mod frontend_helper;
pub mod incentive;
pub mod incentive_factory;
pub mod pair;
pub mod querier;
pub mod router;
pub mod swap;
pub mod token;
pub mod trio;

#[cfg(test)]
mod testing;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock_querier;

#[allow(clippy::all)]
mod uints {
    use uint::construct_uint;
    construct_uint! {
        pub struct U256(4);
    }
}

pub use uints::U256;

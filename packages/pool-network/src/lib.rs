pub mod asset;
pub mod factory;
pub mod pair;
pub mod querier;
pub mod router;
pub mod token;

#[cfg(not(target_arch = "wasm32"))]
pub mod mock_querier;

#[cfg(test)]
mod testing;

#[allow(clippy::all)]
mod uints {
    use uint::construct_uint;
    construct_uint! {
        pub struct U256(4);
    }
}

pub use uints::U256;

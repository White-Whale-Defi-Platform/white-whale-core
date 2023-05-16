pub mod contract;
mod error;
pub mod state;

mod claim;
mod execute;
mod funds_validation;
mod queries;
mod weight;

mod migrations;
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod testing;
#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests;

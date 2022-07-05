use cosmwasm_std::{Decimal, Uint128};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fee {
    pub share: Decimal,
}

impl Fee {
    pub fn compute(&self, amount: Uint128) -> Uint128 {
        amount * self.share
    }
}

/// Fees used by the pools on the pool network
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PoolFee {
    pub protocol_fee: Fee,
    pub swap_fee: Fee,
}

/// Fees used by the flashloan vaults on the liquidity hub
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VaultFee {
    pub protocol_fee: Fee,
    pub flash_loan_fee: Fee,
}

use cosmwasm_bignumber::{Decimal256, Uint256};
use cosmwasm_std::{Decimal, StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Fee {
    pub share: Decimal,
}

impl Fee {
    /// Computes the fee for the given amount
    pub fn compute(&self, amount: Uint256) -> Uint256 {
        amount * Decimal256::from(self.share)
    }

    /// Computes the fee for the given amount
    pub fn to_decimal_256(&self) -> Decimal256 {
        Decimal256::from(self.share)
    }

    /// Checks that the given [Fee] is valid, i.e. it's lower or equal to 100%
    pub fn is_valid(&self) -> StdResult<()> {
        if self.share >= Decimal::percent(100) {
            return Err(StdError::generic_err("Invalid fee"));
        }
        Ok(())
    }
}

/// Fees used by the flashloan vaults on the liquidity hub
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VaultFee {
    pub protocol_fee: Fee,
    pub flash_loan_fee: Fee,
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Decimal, StdError, Uint128};

    use crate::fee::Fee;

    #[test]
    fn valid_fee() {
        let fee = Fee {
            share: Decimal::from_ratio(9u128, 10u128),
        };
        let res = fee.is_valid();
        match res {
            Ok(_) => (),
            Err(_) => panic!("this fee shouldn't fail"),
        }

        let fee = Fee {
            share: Decimal::from_ratio(Uint128::new(2u128), Uint128::new(100u128)),
        };
        let res = fee.is_valid();
        match res {
            Ok(_) => (),
            Err(_) => panic!("this fee shouldn't fail"),
        }

        let fee = Fee {
            share: Decimal::zero(),
        };
        let res = fee.is_valid();
        match res {
            Ok(_) => (),
            Err(_) => panic!("this fee shouldn't fail"),
        }
    }

    #[test]
    fn invalid_fee() {
        let fee = Fee {
            share: Decimal::one(),
        };
        assert_eq!(fee.is_valid(), Err(StdError::generic_err("Invalid fee")));

        let fee = Fee {
            share: Decimal::from_ratio(Uint128::new(2u128), Uint128::new(1u128)),
        };
        assert_eq!(fee.is_valid(), Err(StdError::generic_err("Invalid fee")));
    }
}

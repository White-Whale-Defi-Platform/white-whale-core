use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Decimal256, StdError, StdResult, Uint256};

#[cw_serde]
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
#[cw_serde]
pub struct VaultFee {
    pub protocol_fee: Fee,
    pub flash_loan_fee: Fee,
    pub burn_fee: Fee,
}

impl VaultFee {
    /// Checks that the given [VaultFee] is valid, i.e. the fees provided are valid, and they don't
    /// exceed 100% together
    pub fn is_valid(&self) -> StdResult<()> {
        self.protocol_fee.is_valid()?;
        self.flash_loan_fee.is_valid()?;
        self.burn_fee.is_valid()?;

        if self
            .protocol_fee
            .share
            .checked_add(self.flash_loan_fee.share)?
            .checked_add(self.burn_fee.share)?
            >= Decimal::percent(100)
        {
            return Err(StdError::generic_err("Invalid fees"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Decimal, StdError, Uint128};

    use crate::fee::{Fee, VaultFee};

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

    #[test]
    fn vault_fee() {
        let vault_fee = VaultFee {
            protocol_fee: Fee {
                share: Decimal::percent(50),
            },
            flash_loan_fee: Fee {
                share: Decimal::percent(50),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        };
        assert_eq!(
            vault_fee.is_valid(),
            Err(StdError::generic_err("Invalid fees"))
        );

        let vault_fee = VaultFee {
            protocol_fee: Fee {
                share: Decimal::percent(200),
            },
            flash_loan_fee: Fee {
                share: Decimal::percent(20),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        };
        assert_eq!(
            vault_fee.is_valid(),
            Err(StdError::generic_err("Invalid fee"))
        );

        let vault_fee = VaultFee {
            protocol_fee: Fee {
                share: Decimal::percent(20),
            },
            flash_loan_fee: Fee {
                share: Decimal::percent(200),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        };
        assert_eq!(
            vault_fee.is_valid(),
            Err(StdError::generic_err("Invalid fee"))
        );

        let vault_fee = VaultFee {
            protocol_fee: Fee {
                share: Decimal::percent(20),
            },
            flash_loan_fee: Fee {
                share: Decimal::percent(20),
            },
            burn_fee: Fee {
                share: Decimal::percent(200),
            },
        };
        assert_eq!(
            vault_fee.is_valid(),
            Err(StdError::generic_err("Invalid fee"))
        );

        let vault_fee = VaultFee {
            protocol_fee: Fee {
                share: Decimal::percent(20),
            },
            flash_loan_fee: Fee {
                share: Decimal::percent(60),
            },
            burn_fee: Fee {
                share: Decimal::percent(20),
            },
        };
        assert_eq!(
            vault_fee.is_valid(),
            Err(StdError::generic_err("Invalid fees"))
        );

        let vault_fee = VaultFee {
            protocol_fee: Fee {
                share: Decimal::percent(20),
            },
            flash_loan_fee: Fee {
                share: Decimal::percent(60),
            },
            burn_fee: Fee {
                share: Decimal::zero(),
            },
        };
        assert_eq!(vault_fee.is_valid(), Ok(()));
    }
}

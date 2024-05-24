pub mod fees {
    use cosmwasm_std::Decimal;

    use white_whale_std::fee::{Fee, PoolFee};
    use white_whale_std::vault_manager::VaultFee;

    pub(crate) fn pool_fees_0() -> PoolFee {
        {
            #[cfg(feature = "osmosis")]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                    protocol_fee: Fee {
                        share: Decimal::zero(),
                    },
                    swap_fee: Fee {
                        share: Decimal::zero(),
                    },
                    extra_fees: vec![],
                    osmosis_fee: Fee {
                        share: Decimal::zero(),
                    },
                }
            }

            #[cfg(not(feature = "osmosis"))]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                    protocol_fee: Fee {
                        share: Decimal::zero(),
                    },
                    swap_fee: Fee {
                        share: Decimal::zero(),
                    },
                    extra_fees: vec![],
                }
            }
        }
    }

    pub(crate) fn pool_fees_005() -> PoolFee {
        {
            #[cfg(feature = "osmosis")]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                    protocol_fee: Fee {
                        share: Decimal::from_ratio(3u128, 10000u128),
                    },
                    swap_fee: Fee {
                        share: Decimal::from_ratio(1u128, 10000u128),
                    },
                    extra_fees: vec![],
                    osmosis_fee: Fee {
                        share: Decimal::from_ratio(1u128, 10000u128),
                    },
                }
            }

            #[cfg(not(feature = "osmosis"))]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                    protocol_fee: Fee {
                        share: Decimal::from_ratio(3u128, 10000u128),
                    },
                    swap_fee: Fee {
                        share: Decimal::from_ratio(2u128, 10000u128),
                    },
                    extra_fees: vec![],
                }
            }
        }
    }

    pub(crate) fn pool_fees_03() -> PoolFee {
        {
            #[cfg(feature = "osmosis")]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                    protocol_fee: Fee {
                        share: Decimal::permille(1),
                    },
                    swap_fee: Fee {
                        share: Decimal::permille(1),
                    },
                    extra_fees: vec![],
                    osmosis_fee: Fee {
                        share: Decimal::permille(1),
                    },
                }
            }

            #[cfg(not(feature = "osmosis"))]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                    protocol_fee: Fee {
                        share: Decimal::permille(1),
                    },
                    swap_fee: Fee {
                        share: Decimal::permille(2),
                    },
                    extra_fees: vec![],
                }
            }
        }
    }

    pub(crate) fn pool_fees_1() -> PoolFee {
        {
            #[cfg(feature = "osmosis")]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::permille(3),
                    },
                    protocol_fee: Fee {
                        share: Decimal::permille(3),
                    },
                    swap_fee: Fee {
                        share: Decimal::permille(3),
                    },
                    extra_fees: vec![],
                    osmosis_fee: Fee {
                        share: Decimal::permille(1),
                    },
                }
            }

            #[cfg(not(feature = "osmosis"))]
            {
                PoolFee {
                    burn_fee: Fee {
                        share: Decimal::permille(3),
                    },
                    protocol_fee: Fee {
                        share: Decimal::permille(3),
                    },
                    swap_fee: Fee {
                        share: Decimal::permille(4),
                    },
                    extra_fees: vec![],
                }
            }
        }
    }

    pub(crate) fn vault_fees_0() -> VaultFee {
        VaultFee {
            protocol_fee: Fee {
                share: Default::default(),
            },
            flash_loan_fee: Fee {
                share: Default::default(),
            },
        }
    }

    pub(crate) fn vault_fees_03() -> VaultFee {
        VaultFee {
            protocol_fee: Fee {
                share: Decimal::permille(1),
            },
            flash_loan_fee: Fee {
                share: Decimal::permille(2),
            },
        }
    }
}

pub mod pools {
    use crate::common::helpers;
    use crate::common::suite::TestingSuite;
    use cosmwasm_std::{coin, Addr};
    use white_whale_std::pool_manager::PoolType;

    /// Creates multiple pools
    pub(crate) fn create_pools(suite: &mut TestingSuite, sender: Addr) {
        suite
            .create_pool(
                sender.clone(),
                vec!["uwhale", "uusdc"],
                vec![6, 6],
                helpers::fees::pool_fees_0(),
                PoolType::ConstantProduct,
                Some("uwhale-usdc-free".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec!["uwhale", "uusdc"],
                vec![6, 6],
                helpers::fees::pool_fees_03(),
                PoolType::ConstantProduct,
                Some("uwhale-usdc-cheap".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec!["uwhale", "uusdc"],
                vec![6, 6],
                helpers::fees::pool_fees_1(),
                PoolType::ConstantProduct,
                Some("uwhale-usdc-expensive".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec!["uwhale", "uosmo"],
                vec![6, 6],
                helpers::fees::pool_fees_03(),
                PoolType::ConstantProduct,
                Some("uwhale-uosmo-cheap".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec![
                    "uusdc",
                    "uusdt",
                    "ibc/BEFB9AB13AB43157A0AF6254AD4B1F565AC0CA0C1760B8339BE7B9E2996F7752",
                ],
                vec![6, 6, 6],
                helpers::fees::pool_fees_005(),
                PoolType::StableSwap { amp: 85u64 },
                Some("3pool-stable".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec!["uusdc", "uusdt"],
                vec![6, 6],
                helpers::fees::pool_fees_005(),
                PoolType::StableSwap { amp: 85u64 },
                Some("2pool-stable".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec!["uwhale", "inj"],
                vec![6, 18],
                helpers::fees::pool_fees_03(),
                PoolType::ConstantProduct,
                Some("uwhale-inj".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec!["uwhale", "btc"],
                vec![6, 8],
                helpers::fees::pool_fees_03(),
                PoolType::ConstantProduct,
                Some("uwhale-btc".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_pool(
                sender.clone(),
                vec!["peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5", "uusdc"],
                vec![6, 6],
                helpers::fees::pool_fees_03(),
                PoolType::ConstantProduct,
                Some("peggy-uusdc".to_string()),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            );
    }
}

pub mod vaults {
    use crate::common::helpers;
    use crate::common::suite::TestingSuite;
    use cosmwasm_std::{coin, Addr};

    /// Creates multiple vaults
    pub(crate) fn create_vaults(suite: &mut TestingSuite, sender: Addr) {
        suite
            .create_vault(
                sender.clone(),
                "uwhale",
                Some("uwhale-free".to_string()),
                helpers::fees::vault_fees_0(),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_vault(
                sender.clone(),
                "uwhale",
                Some("whale-cheap".to_string()),
                helpers::fees::vault_fees_03(),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .create_vault(
                sender.clone(),
                "uusdc",
                Some("uusdc-vault".to_string()),
                helpers::fees::vault_fees_03(),
                vec![coin(1_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            );
    }
}

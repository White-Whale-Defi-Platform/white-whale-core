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
    use std::cell::RefCell;

    use cosmwasm_std::{coin, Addr};

    use white_whale_std::pool_manager::{PoolType, SwapOperation, SwapRoute};

    use crate::common::helpers;
    use crate::common::suite::TestingSuite;

    /// Creates multiple pools
    pub(crate) fn create_pools(suite: &mut TestingSuite, sender: Addr) {
        suite
            .create_pool(
                sender.clone(),
                vec!["uwhale", "uusdc"],
                vec![6, 6],
                helpers::fees::pool_fees_0(),
                PoolType::ConstantProduct,
                Some("uwhale-uusdc-free".to_string()),
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
                Some("uwhale-uusdc".to_string()),
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
                Some("uwhale-uusdc-expensive".to_string()),
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
                Some("uusdc-uusdt".to_string()),
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

        // add liquidity
        suite
            .provide_liquidity(
                &sender,
                "uwhale-uusdc-free".to_string(),
                None,
                None,
                None,
                None,
                vec![coin(100_000_000, "uwhale"), coin(100_000_000, "uusdc")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "uwhale-uusdc".to_string(),
                None,
                None,
                None,
                None,
                vec![coin(100_000_000, "uwhale"), coin(100_000_000, "uusdc")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "uwhale-uusdc-expensive".to_string(),
                None,
                None,
                None,
                None,
                vec![coin(100_000_000, "uwhale"), coin(100_000_000, "uusdc")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "uwhale-uosmo-cheap".to_string(),
                None,
                None,
                None,
                None,
                vec![coin(100_000_000, "uwhale"), coin(100_000_000, "uosmo")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "3pool-stable".to_string(),
                None,
                None,
                None,
                None,
                vec![
                    coin(100_000_000, "uusdc"),
                    coin(100_000_000, "uusdt"),
                    coin(
                        100_000_000,
                        "ibc/BEFB9AB13AB43157A0AF6254AD4B1F565AC0CA0C1760B8339BE7B9E2996F7752",
                    ),
                ],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "uusdc-uusdt".to_string(),
                None,
                None,
                None,
                None,
                vec![coin(100_000_000, "uusdc"), coin(100_000_000, "uusdt")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "uwhale-inj".to_string(),
                None,
                None,
                None,
                None,
                vec![
                    coin(100_000_000, "uwhale"),
                    coin(100_000_000_000_000_000_000, "inj"),
                ],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "uwhale-btc".to_string(),
                None,
                None,
                None,
                None,
                vec![coin(100_000_000, "uwhale"), coin(10_000_000_000, "btc")],
                |result| {
                    result.unwrap();
                },
            )
            .provide_liquidity(
                &sender,
                "peggy-uusdc".to_string(),
                None,
                None,
                None,
                None,
                vec![
                    coin(
                        100_000_000,
                        "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5",
                    ),
                    coin(100_000_000, "uusdc"),
                ],
                |result| {
                    result.unwrap();
                },
            );

        // add swap routes
        suite
            .add_swap_routes(
                sender.clone(),
                vec![
                    SwapRoute {
                        offer_asset_denom: "uusdc".to_string(),
                        ask_asset_denom: "uwhale".to_string(),
                        swap_operations: vec![
                            SwapOperation::WhaleSwap {
                                token_in_denom: "uusdc".to_string(),
                                token_out_denom: "uwhale".to_string(),
                                pool_identifier: "uwhale-uusdc".to_string(),
                            }
                        ],
                    },
                    SwapRoute {
                        offer_asset_denom: "uosmo".to_string(),
                        ask_asset_denom: "uwhale".to_string(),
                        swap_operations: vec![
                            SwapOperation::WhaleSwap {
                                token_in_denom: "uosmo".to_string(),
                                token_out_denom: "uwhale".to_string(),
                                pool_identifier: "uwhale-uosmo-cheap".to_string(),
                            }
                        ],
                    },
                    SwapRoute {
                        offer_asset_denom: "uusdt".to_string(),
                        ask_asset_denom: "uwhale".to_string(),
                        swap_operations: vec![
                            SwapOperation::WhaleSwap {
                                token_in_denom: "uusdt".to_string(),
                                token_out_denom: "uusdc".to_string(),
                                pool_identifier: "uusdc-uusdt".to_string(),
                            },
                            SwapOperation::WhaleSwap {
                                token_in_denom: "uusdc".to_string(),
                                token_out_denom: "uwhale".to_string(),
                                pool_identifier: "uwhale-uusdc".to_string(),
                            },
                        ],
                    },
                    SwapRoute {
                        offer_asset_denom: "ibc/BEFB9AB13AB43157A0AF6254AD4B1F565AC0CA0C1760B8339BE7B9E2996F7752".to_string(),
                        ask_asset_denom: "uwhale".to_string(),
                        swap_operations: vec![
                            SwapOperation::WhaleSwap {
                                token_in_denom: "ibc/BEFB9AB13AB43157A0AF6254AD4B1F565AC0CA0C1760B8339BE7B9E2996F7752".to_string(),
                                token_out_denom: "uusdc".to_string(),
                                pool_identifier: "3pool-stable".to_string(),
                            },
                            SwapOperation::WhaleSwap {
                                token_in_denom: "uusdc".to_string(),
                                token_out_denom: "uwhale".to_string(),
                                pool_identifier: "uwhale-uusdc".to_string(),
                            },
                        ],
                    },
                    SwapRoute {
                        offer_asset_denom: "inj".to_string(),
                        ask_asset_denom: "uwhale".to_string(),
                        swap_operations: vec![
                            SwapOperation::WhaleSwap {
                                token_in_denom: "inj".to_string(),
                                token_out_denom: "uwhale".to_string(),
                                pool_identifier: "uwhale-inj".to_string(),
                            }
                        ],
                    },
                    SwapRoute {
                        offer_asset_denom: "btc".to_string(),
                        ask_asset_denom: "uwhale".to_string(),
                        swap_operations: vec![
                            SwapOperation::WhaleSwap {
                                token_in_denom: "btc".to_string(),
                                token_out_denom: "uwhale".to_string(),
                                pool_identifier: "uwhale-btc".to_string(),
                            }
                        ],
                    },
                    SwapRoute {
                        offer_asset_denom: "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(),
                        ask_asset_denom: "uwhale".to_string(),
                        swap_operations: vec![
                            SwapOperation::WhaleSwap {
                                token_in_denom: "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(),
                                token_out_denom: "uusdc".to_string(),
                                pool_identifier: "peggy-uusdc".to_string(),
                            },
                            SwapOperation::WhaleSwap {
                                token_in_denom: "uusdc".to_string(),
                                token_out_denom: "uwhale".to_string(),
                                pool_identifier: "uwhale-uusdc".to_string(),
                            },
                        ],
                    },
                ],
                |result| {
                    result.unwrap();
                },
            );

        let pool_identifiers = RefCell::new(vec![]);

        pool_identifiers.borrow_mut().extend(vec![
            "uwhale-uusdc-free".to_string(),
            "uwhale-uusdc".to_string(),
            "uwhale-uusdc-expensive".to_string(),
            "uwhale-uosmo-cheap".to_string(),
            "3pool-stable".to_string(),
            "uusdc-uusdt".to_string(),
            "uwhale-inj".to_string(),
            "uwhale-btc".to_string(),
            "peggy-uusdc".to_string(),
        ]);

        suite.pool_identifiers = pool_identifiers.into_inner();
    }
}

pub mod vaults {
    use std::cell::RefCell;

    use cosmwasm_std::{coin, Addr};

    use crate::common::helpers;
    use crate::common::suite::TestingSuite;

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

        let vault_identifiers = RefCell::new(vec![]);

        suite.query_vaults(None, None, |result| {
            let vaults_response = result.unwrap();
            for vault in vaults_response.vaults {
                vault_identifiers.borrow_mut().push(vault.identifier);
            }
        });

        suite.vault_identifiers = vault_identifiers.into_inner();
    }

    pub(crate) fn add_vault_liquidity(_p0: &mut TestingSuite, _p1: Addr) {
        //todo
    }
}

pub mod incentives {
    use std::cell::RefCell;

    use cosmwasm_std::{coin, Addr};

    use white_whale_std::incentive_manager::{IncentiveAction, IncentiveParams};

    use crate::common::suite::TestingSuite;

    pub(crate) fn create_incentives(suite: &mut TestingSuite, sender: Addr) {
        let lp_tokens = RefCell::new(suite.pool_identifiers.clone());

        suite
            .manage_incentive(
                sender.clone(),
                IncentiveAction::Fill {
                    params: IncentiveParams {
                        lp_denom: lp_tokens.borrow()[0].clone(),
                        start_epoch: Some(3),
                        preliminary_end_epoch: None,
                        curve: None,
                        incentive_asset: coin(1_000_000u128, "uwhale"),
                        incentive_identifier: None,
                    },
                },
                vec![coin(1_001_000u128, "uwhale")],
                |result| {
                    result.unwrap();
                },
            )
            .manage_incentive(
                sender.clone(),
                IncentiveAction::Fill {
                    params: IncentiveParams {
                        lp_denom: lp_tokens.borrow()[1].clone(),
                        start_epoch: Some(3),
                        preliminary_end_epoch: None,
                        curve: None,
                        incentive_asset: coin(1_000_000u128, "uosmo"),
                        incentive_identifier: None,
                    },
                },
                vec![coin(1_000u128, "uwhale"), coin(1_000_000u128, "uosmo")],
                |result| {
                    result.unwrap();
                },
            )
            .manage_incentive(
                sender.clone(),
                IncentiveAction::Fill {
                    params: IncentiveParams {
                        lp_denom: lp_tokens.borrow()[3].clone(),
                        start_epoch: Some(3),
                        preliminary_end_epoch: None,
                        curve: None,
                        incentive_asset: coin(1_000_000u128, "uosmo"),
                        incentive_identifier: None,
                    },
                },
                vec![coin(1_000u128, "uwhale"), coin(1_000_000u128, "uosmo")],
                |result| {
                    result.unwrap();
                },
            )
            .manage_incentive(
                sender.clone(),
                IncentiveAction::Fill {
                    params: IncentiveParams {
                        lp_denom: lp_tokens.borrow()[6].clone(),
                        start_epoch: Some(8),
                        preliminary_end_epoch: None,
                        curve: None,
                        incentive_asset: coin(3_000u128, "btc"),
                        incentive_identifier: None,
                    },
                },
                vec![coin(1_000u128, "uwhale"), coin(3_000u128, "btc")],
                |result| {
                    result.unwrap();
                },
            )
            .manage_incentive(
                sender.clone(),
                IncentiveAction::Fill {
                    params: IncentiveParams {
                        lp_denom: lp_tokens.borrow()[6].clone(),
                        start_epoch: Some(8),
                        preliminary_end_epoch: None,
                        curve: None,
                        incentive_asset: coin(3_000u128, "uosmo"),
                        incentive_identifier: None,
                    },
                },
                vec![coin(1_000u128, "uwhale"), coin(3_000u128, "uosmo")],
                |result| {
                    result.unwrap();
                },
            )
            .manage_incentive(
                sender.clone(),
                IncentiveAction::Fill {
                    params: IncentiveParams {
                        lp_denom: lp_tokens.borrow()[6].clone(),
                        start_epoch: Some(10),
                        preliminary_end_epoch: None,
                        curve: None,
                        incentive_asset: coin(10_000u128, "uusdc"),
                        incentive_identifier: None,
                    },
                },
                vec![coin(1_000u128, "uwhale"), coin(10_000u128, "uusdc")],
                |result| {
                    result.unwrap();
                },
            )
            .manage_incentive(
                sender.clone(),
                IncentiveAction::Fill {
                    params: IncentiveParams {
                        lp_denom: lp_tokens.borrow()[6].clone(),
                        start_epoch: Some(10),
                        preliminary_end_epoch: None,
                        curve: None,
                        incentive_asset: coin(
                            100_000u128,
                            "factory/migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40/ampWHALE",
                        ),
                        incentive_identifier: None,
                    },
                },
                vec![
                    coin(1_000u128, "uwhale"),
                    coin(
                        100_000u128,
                        "factory/migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40/ampWHALE",
                    ),
                ],
                |result| {
                    result.unwrap();
                },
            );
    }
}

use std::collections::HashMap;

use cosmwasm_std::{
    coin, coins, to_binary, Addr, BankMsg, BlockInfo, Coin, Decimal, Timestamp, Uint128, Uint256,
    Uint64,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ExecuteMsg, MinterResponse};
use cw_multi_test::Executor;

use white_whale::fee::{Fee, VaultFee};
use white_whale::fee_collector::ExecuteMsg::{
    AggregateFees, CollectFees, ForwardFees, UpdateConfig,
};
use white_whale::fee_collector::{
    Contract, ContractType, FactoryType, FeesFor, InstantiateMsg, QueryMsg,
};
use white_whale::fee_distributor::ExecuteMsg::NewEpoch;
use white_whale::fee_distributor::{Epoch, EpochConfig, EpochResponse};
use white_whale::pool_network::asset::{Asset, AssetInfo, PairType};
use white_whale::pool_network::factory::ExecuteMsg::{AddNativeTokenDecimals, CreatePair};
use white_whale::pool_network::factory::PairsResponse;
use white_whale::pool_network::pair::{PoolFee, PoolResponse, ProtocolFeesResponse};
use white_whale::pool_network::router::{SwapOperation, SwapRoute};
use white_whale::vault_network::vault_factory::ExecuteMsg;
use white_whale::{pool_network, vault_network};

use crate::tests::common_integration::{
    increase_allowance, mock_app, mock_app_with_balance, mock_creator,
    store_dummy_flash_loan_contract, store_fee_collector_code, store_fee_distributor_code,
    store_pair_code, store_pool_factory_code, store_pool_router_code, store_token_code,
    store_trio_code, store_vault_code, store_vault_factory_code, store_whale_lair_code,
};
use crate::ContractError;

#[test]
fn collect_all_factories_cw20_fees_successfully() {
    const TOKEN_AMOUNT: u8 = 4;

    let mut app = mock_app();
    let creator = mock_creator();

    let fee_collector_id = store_fee_collector_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: None,
            pool_factory: None,
            vault_factory: None,
        },
        &[],
    )
    .unwrap();

    // Create few tokens to create pools with
    let mut cw20_tokens: Vec<Addr> = Vec::new();

    for i in 0..TOKEN_AMOUNT {
        let token_address = app
            .instantiate_contract(
                token_id,
                creator.clone().sender,
                &pool_network::token::InstantiateMsg {
                    name: format!("token{}", i),
                    symbol: format!("token{}", (i + b'a') as char),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: creator.clone().sender.to_string(),
                        amount: Uint128::new(1_000_000_000_000u128),
                    }],
                    mint: Some(MinterResponse {
                        minter: creator.clone().sender.to_string(),
                        cap: None,
                    }),
                },
                &[],
                "cw20 token",
                None,
            )
            .unwrap();

        cw20_tokens.push(token_address.clone());
    }

    // Create few pools
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for i in 0..TOKEN_AMOUNT - 1 {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: cw20_tokens[i as usize].to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: cw20_tokens[i as usize + 1].to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        let pair_address = Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        );

        pair_tokens.push(pair_address.clone());
    }

    // Increase allowance for the tokens on the pools
    for i in 0..TOKEN_AMOUNT {
        // first and last token in array exist only in one pool, while the rest in two
        if i == 0 {
            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i as usize].clone(),
                pair_tokens[i as usize].clone(),
            );
        } else if i == TOKEN_AMOUNT - 1 {
            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i as usize].clone(),
                pair_tokens[i as usize - 1].clone(),
            );
        } else {
            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i as usize].clone(),
                pair_tokens[i as usize].clone(),
            );

            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i as usize].clone(),
                pair_tokens[i as usize - 1].clone(),
            );
        }
    }

    // Provide liquidity into pools
    for i in 0..TOKEN_AMOUNT - 1 {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i as usize].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cw20_tokens[i as usize].to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cw20_tokens[i as usize + 1].to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[],
        )
        .unwrap();
    }

    let mut assets_collected: HashMap<String, Asset> = HashMap::new();

    // Perform some swaps
    for i in 1..TOKEN_AMOUNT - 1 {
        app.execute_contract(
            creator.sender.clone(),
            cw20_tokens[i as usize].clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair_tokens[i as usize - 1].to_string(),
                amount: Uint128::new(100_000_000u128),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            creator.sender.clone(),
            cw20_tokens[i as usize].clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair_tokens[i as usize].to_string(),
                amount: Uint128::new(200_000_000_000u128),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        // Verify the fees are being collected
        let query_protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_tokens[i as usize - 1],
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        let protocol_fees = query_protocol_fees_res
            .fees
            .iter()
            .find(|&asset| {
                let asset_addr = match &asset.info {
                    AssetInfo::Token { contract_addr } => contract_addr,
                    AssetInfo::NativeToken { .. } => panic!("no native tokens in this test"),
                };
                // fees are collected in the token opposite of the one you swap
                asset_addr.to_string() != cw20_tokens[i as usize]
            })
            .unwrap();

        accumulate_fee(&mut assets_collected, protocol_fees);

        assert!(protocol_fees.amount > Uint128::zero());

        let query_protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_tokens[i as usize],
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        let protocol_fees = query_protocol_fees_res
            .fees
            .iter()
            .find(|&asset| {
                let asset_addr = match &asset.info {
                    AssetInfo::Token { contract_addr } => contract_addr,
                    AssetInfo::NativeToken { .. } => panic!("no native tokens in this test"),
                };
                // fees are collected in the token opposite of the one you swap
                asset_addr.to_string() != cw20_tokens[i as usize]
            })
            .unwrap();

        accumulate_fee(&mut assets_collected, protocol_fees);

        // Verify fees are being collected
        assert!(protocol_fees.amount > Uint128::zero());
    }

    // Make sure the fee collector's balance for the tokens in which fees were collected is zero
    for (asset_addr, _) in assets_collected.clone() {
        let balance_res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                &asset_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: fee_collector_address.clone().to_string(),
                },
            )
            .unwrap();

        assert_eq!(balance_res.balance, Uint128::zero());
    }

    // Collect the fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &CollectFees {
            collect_fees_for: FeesFor::Factory {
                factory_addr: pool_factory_address.to_string(),
                factory_type: FactoryType::Pool {
                    start_after: None,
                    limit: Some(u32::try_from(TOKEN_AMOUNT).unwrap()),
                },
            },
        },
        &[],
    )
    .unwrap();

    // Make sure the fee collector's balance for the tokens in which fees were collected increased,
    // and matches the amount the pool reported to have collected
    // Aggregate the fees collected by the fee collector
    let ask_asset = AssetInfo::Token {
        contract_addr: cw20_tokens[0].to_string(),
    };
    let mut ask_asset_original_balance = Uint128::zero();
    for (asset_addr, asset) in assets_collected.clone() {
        let balance_res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                &asset_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: fee_collector_address.clone().to_string(),
                },
            )
            .unwrap();
        assert!(balance_res.balance > Uint128::zero());
        assert_eq!(balance_res.balance, asset.amount);

        if asset.info == ask_asset {
            ask_asset_original_balance = balance_res.balance;
        }
    }

    // Make sure protocol fees in the pools are zero, as they have been collected
    for pair_token in pair_tokens {
        let protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_token.clone(),
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        for fee in protocol_fees_res.fees {
            assert_eq!(fee.amount, Uint128::zero());
        }
    }

    // Aggregate the fees collected by the fee collector
    // Add swap routes to the router to aggregate fees
    for i in 1..TOKEN_AMOUNT {
        let mut swap_routes: Vec<SwapRoute> = vec![];
        let mut swap_operations: Vec<SwapOperation> = vec![];

        for i in (0..i).rev() {
            let swap_operation = SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: cw20_tokens[i as usize + 1].to_string(),
                },
                ask_asset_info: AssetInfo::Token {
                    contract_addr: cw20_tokens[i as usize].to_string(),
                },
            };
            swap_operations.push(swap_operation);
        }

        let swap_route = SwapRoute {
            offer_asset_info: AssetInfo::Token {
                contract_addr: cw20_tokens[i as usize].to_string(),
            },
            ask_asset_info: ask_asset.clone(),
            swap_operations,
        };
        swap_routes.push(swap_route);

        app.execute_contract(
            creator.sender.clone(),
            pool_router_address.clone(),
            &pool_network::router::ExecuteMsg::AddSwapRoutes { swap_routes },
            &[],
        )
        .unwrap();
    }

    // Aggregate fees
    app.execute_contract(
        creator.sender,
        fee_collector_address.clone(),
        &AggregateFees {
            asset_info: ask_asset,
            aggregate_fees_for: FeesFor::Factory {
                factory_addr: pool_factory_address.to_string(),
                factory_type: FactoryType::Pool {
                    start_after: None,
                    limit: Some(u32::try_from(TOKEN_AMOUNT).unwrap()),
                },
            },
        },
        &[],
    )
    .unwrap();

    // Make sure the balances of aggregated assets are zero
    // remove the ask asset from the list of assets collected, before making sure their balances is zero
    assets_collected.remove(&cw20_tokens[0].to_string());

    for (asset_addr, _) in assets_collected {
        let balance_res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                &asset_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: fee_collector_address.clone().to_string(),
                },
            )
            .unwrap();
        assert_eq!(balance_res.balance, Uint128::zero());
    }

    // check ask_asset balance, should be greater than the initial one
    let balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            &cw20_tokens[0],
            &cw20::Cw20QueryMsg::Balance {
                address: fee_collector_address.to_string(),
            },
        )
        .unwrap();
    assert!(balance_res.balance > ask_asset_original_balance);
}

#[test]
fn collect_cw20_fees_for_specific_contracts_successfully() {
    const TOKEN_AMOUNT: usize = 10;
    const POOLS_TO_COLLECT_FEES_FROM: usize = 3;

    let mut app = mock_app();
    let creator = mock_creator();

    let fee_collector_id = store_fee_collector_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    // Create few tokens to create pools with
    let mut cw20_tokens: Vec<Addr> = Vec::new();
    for i in 0..TOKEN_AMOUNT {
        let token_address = app
            .instantiate_contract(
                token_id,
                creator.clone().sender,
                &pool_network::token::InstantiateMsg {
                    name: format!("token{}", i),
                    symbol: "token".to_string(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: creator.clone().sender.to_string(),
                        amount: Uint128::new(1_000_000_000_000u128),
                    }],
                    mint: Some(MinterResponse {
                        minter: creator.clone().sender.to_string(),
                        cap: None,
                    }),
                },
                &[],
                "cw20 token",
                None,
            )
            .unwrap();

        cw20_tokens.push(token_address);
    }

    // Create few pools
    let mut pair_tokens: Vec<Addr> = Vec::new();

    for i in 0..TOKEN_AMOUNT - 1 {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::Token {
                            contract_addr: cw20_tokens[i].to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: cw20_tokens[i + 1].to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        pair_tokens.push(Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        ));
    }

    // Increase allowance for the tokens on the pools
    for i in 0..TOKEN_AMOUNT {
        // first and last token in array exist only in one pool, while the rest in two
        if i == 0 {
            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i].clone(),
                pair_tokens[i].clone(),
            );
        } else if i == TOKEN_AMOUNT - 1 {
            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i].clone(),
                pair_tokens[i - 1].clone(),
            );
        } else {
            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i].clone(),
                pair_tokens[i].clone(),
            );

            increase_allowance(
                &mut app,
                creator.sender.clone(),
                cw20_tokens[i].clone(),
                pair_tokens[i - 1].clone(),
            );
        }
    }

    // Provide liquidity into pools
    for i in 0..TOKEN_AMOUNT - 1 {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cw20_tokens[i].to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cw20_tokens[i + 1].to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[],
        )
        .unwrap();
    }

    // Verify fees for a pool via the collector's query, should be zero at this stage
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Contracts {
                    contracts: vec![Contract {
                        address: pair_tokens[1].clone().to_string(),
                        contract_type: ContractType::Pool {},
                    }],
                },
                all_time: None,
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 2usize);
    for asset in fee_collector_fees_query {
        assert_eq!(asset.amount, Uint128::zero());
    }

    // Perform some swaps
    let mut assets_collected: HashMap<String, Asset> = HashMap::new();
    for i in 1..TOKEN_AMOUNT - 1 {
        app.execute_contract(
            creator.sender.clone(),
            cw20_tokens[i].clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair_tokens[i - 1].to_string(),
                amount: Uint128::new(100_000_000u128),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            creator.sender.clone(),
            cw20_tokens[i].clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair_tokens[i].to_string(),
                amount: Uint128::new(200_000_000_000u128),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        // Verify the fees are being collected
        let query_protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_tokens[i - 1],
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        let protocol_fees = query_protocol_fees_res
            .fees
            .iter()
            .find(|&asset| {
                let asset_addr = match &asset.info {
                    AssetInfo::Token { contract_addr } => contract_addr,
                    AssetInfo::NativeToken { .. } => panic!("no native tokens in this test"),
                };
                // fees are collected in the token opposite of the one you swap
                asset_addr.to_string() != cw20_tokens[i]
            })
            .unwrap();

        accumulate_fee(&mut assets_collected, protocol_fees);

        assert!(protocol_fees.amount > Uint128::zero());

        let query_protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_tokens[i],
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        let protocol_fees = query_protocol_fees_res
            .fees
            .iter()
            .find(|&asset| {
                let asset_addr = match &asset.info {
                    AssetInfo::Token { contract_addr } => contract_addr,
                    AssetInfo::NativeToken { .. } => panic!("no native tokens in this test"),
                };
                // fees are collected in the token opposite of the one you swap
                asset_addr.to_string() != cw20_tokens[i]
            })
            .unwrap();

        accumulate_fee(&mut assets_collected, protocol_fees);

        // Verify fees are being collected
        assert!(protocol_fees.amount > Uint128::zero());
    }

    // Verify fees for a pool via the collector's query, should not be zero at this stage
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Contracts {
                    contracts: vec![Contract {
                        address: pair_tokens[1].clone().to_string(),
                        contract_type: ContractType::Pool {},
                    }],
                },
                all_time: None,
            },
        )
        .unwrap();
    for asset in fee_collector_fees_query {
        assert!(asset.amount > Uint128::zero());
    }

    // Make sure the fee collector's balance for the tokens in which fees were collected is zero
    for (asset_addr, _) in assets_collected.clone() {
        let balance_res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                &asset_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: fee_collector_address.clone().to_string(),
                },
            )
            .unwrap();

        assert_eq!(balance_res.balance, Uint128::zero());
    }

    // Collect the fees for specific contracts
    // get first POOLS_TO_COLLECT_FEES_FROM pools
    // for simplicity, drop the first pool as the first token is not being swapped/collected
    pair_tokens.remove(0);
    let pair_tokens: Vec<String> = pair_tokens
        .chunks(POOLS_TO_COLLECT_FEES_FROM)
        .next()
        .unwrap()
        .iter()
        .map(|address| address.clone().to_string())
        .collect();

    // store the tokens in the pools on the filtered pair_tokens
    let mut tokens_in_filtered_pairs: HashMap<String, Asset> = HashMap::new();
    for pair_token in pair_tokens.clone() {
        let pool_res: PoolResponse = app
            .wrap()
            .query_wasm_smart(&pair_token, &pool_network::pair::QueryMsg::Pool {})
            .unwrap();

        for asset in pool_res.assets {
            tokens_in_filtered_pairs.insert(asset.clone().get_id(), asset.clone());
        }
    }

    assert_eq!(
        tokens_in_filtered_pairs.len(),
        POOLS_TO_COLLECT_FEES_FROM + 1
    );

    // collect the fees
    let mut contracts: Vec<Contract> = Vec::new();

    for pair in pair_tokens.clone() {
        contracts.push(Contract {
            address: pair.clone().to_string(),
            contract_type: ContractType::Pool {},
        });
    }

    app.execute_contract(
        creator.sender,
        fee_collector_address.clone(),
        &CollectFees {
            collect_fees_for: FeesFor::Contracts { contracts },
        },
        &[],
    )
    .unwrap();

    // Make sure the fee collector's balance for the tokens in which fees were collected increased,
    // and matches the amount the pool reported to have collected
    for (asset_addr, _) in assets_collected.clone() {
        let balance_res: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                &asset_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: fee_collector_address.clone().to_string(),
                },
            )
            .unwrap();

        // if balance is higher than 0, then fees were collected
        if balance_res.balance > Uint128::zero() {
            tokens_in_filtered_pairs.remove(&asset_addr);
        }
    }

    // tokens_in_filtered_pairs should be empty as all the tokens that collected fees were removed
    // from it above
    assert_eq!(tokens_in_filtered_pairs.len(), 0usize);

    // Verify fees for a pool via the collector's query
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address,
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Contracts {
                    contracts: vec![Contract {
                        address: pair_tokens[1].to_string(),
                        contract_type: ContractType::Pool {},
                    }],
                },
                all_time: None,
            },
        )
        .unwrap();
    for asset in fee_collector_fees_query {
        assert_eq!(asset.amount, Uint128::zero());
    }
}

#[test]
fn collect_pools_native_fees_successfully() {
    const TOKEN_AMOUNT: u8 = 3;

    let creator = mock_creator();
    let balances = vec![(
        creator.clone().sender,
        coins(1_000_000_000u128, "native".to_string()),
    )];

    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: None,
            pool_factory: None,
            vault_factory: None,
        },
        &[],
    )
    .unwrap();

    // add native token to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "native".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "native".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    // Create few tokens to create pools with
    let mut cw20_tokens: Vec<Addr> = Vec::new();
    for i in 0..TOKEN_AMOUNT {
        let symbol = format!("token{}", (i + b'a') as char);
        let token_address = app
            .instantiate_contract(
                token_id,
                creator.clone().sender,
                &pool_network::token::InstantiateMsg {
                    name: format!("token{}", i),
                    symbol: symbol.clone(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: creator.clone().sender.to_string(),
                        amount: Uint128::new(1_000_000_000_000u128),
                    }],
                    mint: Some(MinterResponse {
                        minter: creator.clone().sender.to_string(),
                        cap: None,
                    }),
                },
                &[],
                "cw20 token",
                None,
            )
            .unwrap();

        cw20_tokens.push(token_address);
    }

    // Create few pools
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for cw20_token in cw20_tokens.clone() {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "native".to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: cw20_token.to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        let pair_address = Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        );
        pair_tokens.push(pair_address);
    }

    // Increase allowance for the tokens on the pools
    for (i, cw20_token) in cw20_tokens.clone().iter().enumerate() {
        increase_allowance(
            &mut app,
            creator.sender.clone(),
            cw20_token.clone(),
            pair_tokens[i].clone(),
        );
    }

    // Provide liquidity into pools
    for (i, cw20_token) in cw20_tokens.clone().iter().enumerate() {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "native".to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cw20_token.to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[Coin {
                denom: "native".to_string(),
                amount: Uint128::new(500_000u128),
            }],
        )
        .unwrap();
    }

    // Verify fees for the factory via the collector's query
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Factory {
                    factory_addr: pool_factory_address.to_string(),
                    factory_type: FactoryType::Pool {
                        start_after: None,
                        limit: None,
                    },
                },
                all_time: None,
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 4usize);
    for asset in fee_collector_fees_query {
        assert_eq!(asset.amount, Uint128::zero());
    }

    // Perform some swaps
    let mut assets_collected: HashMap<String, Asset> = HashMap::new();
    for (i, cw20_token) in cw20_tokens.clone().iter().enumerate() {
        // swap native -> cw20
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "native".to_string(),
                    },
                    amount: Uint128::new(200_000_000u128),
                },
                belief_price: None,
                max_spread: None,
                to: None,
            },
            &[Coin {
                denom: "native".to_string(),
                amount: Uint128::new(200_000_000u128),
            }],
        )
        .unwrap();

        // swap cw20 -> native
        app.execute_contract(
            creator.sender.clone(),
            cw20_token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair_tokens[i].to_string(),
                amount: Uint128::new(200_000_000u128),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        // Verify the fees are being collected
        let query_protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_tokens[i],
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        for asset in query_protocol_fees_res.fees {
            assert!(asset.amount > Uint128::zero());
            accumulate_fee(&mut assets_collected, &asset);
        }
    }

    // assert the assets collected are the native token + the tokens created
    assert_eq!(assets_collected.len() as u8, TOKEN_AMOUNT + 1);

    // Make sure the fee collector's balance for the assets in which fees were collected is zero
    for (asset_id, _) in assets_collected.clone() {
        if asset_id == "native" {
            let balance_res = app
                .wrap()
                .query_balance(fee_collector_address.clone().to_string(), "native")
                .unwrap();
            assert_eq!(balance_res.amount, Uint128::zero());
        } else {
            let balance_res: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    &asset_id,
                    &cw20::Cw20QueryMsg::Balance {
                        address: fee_collector_address.clone().to_string(),
                    },
                )
                .unwrap();
            assert_eq!(balance_res.balance, Uint128::zero());
        }
    }

    // Collect the fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &CollectFees {
            collect_fees_for: FeesFor::Factory {
                factory_addr: pool_factory_address.to_string(),
                factory_type: FactoryType::Pool {
                    start_after: None,
                    limit: None,
                },
            },
        },
        &[],
    )
    .unwrap();

    //Query fees for the factory via the collector's query, for all time
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Factory {
                    factory_addr: pool_factory_address.to_string(),
                    factory_type: FactoryType::Pool {
                        start_after: None,
                        limit: None,
                    },
                },
                all_time: Some(true),
            },
        )
        .unwrap();

    // Make sure the fee collector's balance for the assets in which fees were collected increased,
    // and matches the amount the pool reported to have collected
    let ask_asset = AssetInfo::Token {
        contract_addr: cw20_tokens[0].to_string(),
    };
    let mut ask_asset_original_balance = Uint128::zero();
    for (asset_id, asset) in assets_collected.clone() {
        if asset_id == "native" {
            let balance_res = app
                .wrap()
                .query_balance(fee_collector_address.clone().to_string(), "native")
                .unwrap();
            assert!(balance_res.amount > Uint128::zero());
            assert_eq!(balance_res.amount, asset.amount);

            let native_asset = fee_collector_fees_query
                .iter()
                .find(|asset| asset.is_native_token())
                .unwrap();
            assert_eq!(balance_res.amount, native_asset.amount);
        } else {
            let balance_res: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    &asset_id,
                    &cw20::Cw20QueryMsg::Balance {
                        address: fee_collector_address.clone().to_string(),
                    },
                )
                .unwrap();
            assert!(balance_res.balance > Uint128::zero());
            assert_eq!(balance_res.balance, asset.amount);

            let asset_from_query = fee_collector_fees_query
                .iter()
                .find(|&a| a.info == asset.info)
                .unwrap();
            assert_eq!(balance_res.balance, asset_from_query.amount);

            if asset.info == ask_asset {
                ask_asset_original_balance = balance_res.balance;
            }
        }
    }

    // Make sure protocol fees in the pools are zero, as they have been collected
    for pair_token in pair_tokens {
        let protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_token.clone(),
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        for fee in protocol_fees_res.fees {
            assert_eq!(fee.amount, Uint128::zero());
        }
    }

    // Verify protocol fees in the pools are zero via the collector's query
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Factory {
                    factory_addr: pool_factory_address.to_string(),
                    factory_type: FactoryType::Pool {
                        start_after: None,
                        limit: None,
                    },
                },
                all_time: None,
            },
        )
        .unwrap();

    for asset in fee_collector_fees_query {
        assert_eq!(asset.amount, Uint128::zero());
    }

    // Aggregate the fees collected by the fee collector
    // Add swap routes to the router to aggregate fees
    for cw20_token in cw20_tokens.clone() {
        if cw20_token == ask_asset.to_string() {
            continue;
        }

        let swap_operations = vec![
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::Token {
                    contract_addr: cw20_token.to_string(),
                },
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "native".to_string(),
                },
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "native".to_string(),
                },
                ask_asset_info: ask_asset.clone(),
            },
        ];

        let swap_routes: Vec<SwapRoute> = vec![SwapRoute {
            offer_asset_info: AssetInfo::Token {
                contract_addr: cw20_token.to_string(),
            },
            ask_asset_info: ask_asset.clone(),
            swap_operations,
        }];

        app.execute_contract(
            creator.sender.clone(),
            pool_router_address.clone(),
            &pool_network::router::ExecuteMsg::AddSwapRoutes { swap_routes },
            &[],
        )
        .unwrap();
    }

    // add native -> token swap route
    let swap_operations = vec![SwapOperation::TerraSwap {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "native".to_string(),
        },
        ask_asset_info: ask_asset.clone(),
    }];

    let swap_routes: Vec<SwapRoute> = vec![SwapRoute {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "native".to_string(),
        },
        ask_asset_info: ask_asset.clone(),
        swap_operations,
    }];

    app.execute_contract(
        creator.sender.clone(),
        pool_router_address,
        &pool_network::router::ExecuteMsg::AddSwapRoutes { swap_routes },
        &[],
    )
    .unwrap();

    // Aggregate fees
    app.execute_contract(
        creator.sender,
        fee_collector_address.clone(),
        &AggregateFees {
            asset_info: ask_asset,
            aggregate_fees_for: FeesFor::Factory {
                factory_addr: pool_factory_address.to_string(),
                factory_type: FactoryType::Pool {
                    start_after: None,
                    limit: Some(u32::try_from(TOKEN_AMOUNT).unwrap()),
                },
            },
        },
        &[],
    )
    .unwrap();

    // Make sure the balances of aggregated assets are zero
    // remove the ask asset from the list of assets collected, before making sure their balances is zero
    assets_collected.remove(&cw20_tokens[0].to_string());

    for (asset_addr, _) in assets_collected {
        let balance = if asset_addr == "native" {
            let balance_res = app
                .wrap()
                .query_balance(fee_collector_address.clone().to_string(), "native")
                .unwrap();
            balance_res.amount
        } else {
            let balance_res: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    &asset_addr,
                    &cw20::Cw20QueryMsg::Balance {
                        address: fee_collector_address.clone().to_string(),
                    },
                )
                .unwrap();
            balance_res.balance
        };
        assert_eq!(balance, Uint128::zero());
    }

    // check ask_asset balance, should be greater than the initial one
    let balance_res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            &cw20_tokens[0],
            &cw20::Cw20QueryMsg::Balance {
                address: fee_collector_address.to_string(),
            },
        )
        .unwrap();
    assert!(balance_res.balance > ask_asset_original_balance);
}

#[test]
fn collect_fees_with_pagination_successfully() {
    const TOKEN_AMOUNT: usize = 10;

    let creator = mock_creator();
    let balances = vec![(
        creator.clone().sender,
        coins(1_000_000_000u128, "native".to_string()),
    )];

    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    // add native token to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "native".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "native".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    // Create few tokens to create pools with
    let mut cw20_tokens: Vec<Addr> = Vec::new();
    for i in 0..TOKEN_AMOUNT {
        let token_address = app
            .instantiate_contract(
                token_id,
                creator.clone().sender,
                &pool_network::token::InstantiateMsg {
                    name: format!("token{}", i),
                    symbol: "token".to_string(),
                    decimals: 6,
                    initial_balances: vec![Cw20Coin {
                        address: creator.clone().sender.to_string(),
                        amount: Uint128::new(1_000_000_000_000u128),
                    }],
                    mint: Some(MinterResponse {
                        minter: creator.clone().sender.to_string(),
                        cap: None,
                    }),
                },
                &[],
                "cw20 token",
                None,
            )
            .unwrap();

        cw20_tokens.push(token_address);
    }

    // Create few pools
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for cw20_token in cw20_tokens.clone() {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "native".to_string(),
                        },
                        AssetInfo::Token {
                            contract_addr: cw20_token.to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        pair_tokens.push(Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        ));
    }

    // Increase allowance for the tokens on the pools
    for (i, cw20_token) in cw20_tokens.clone().iter().enumerate() {
        increase_allowance(
            &mut app,
            creator.sender.clone(),
            cw20_token.clone(),
            pair_tokens[i].clone(),
        );
    }

    // Provide liquidity into pools
    for (i, cw20_token) in cw20_tokens.clone().iter().enumerate() {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "native".to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::Token {
                            contract_addr: cw20_token.to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[Coin {
                denom: "native".to_string(),
                amount: Uint128::new(500_000u128),
            }],
        )
        .unwrap();
    }

    // Perform some swaps
    let mut assets_collected: HashMap<String, Asset> = HashMap::new();
    for (i, cw20_token) in cw20_tokens.clone().iter().enumerate() {
        // swap native -> cw20
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "native".to_string(),
                    },
                    amount: Uint128::new(200_000_000u128),
                },
                belief_price: None,
                max_spread: None,
                to: None,
            },
            &[Coin {
                denom: "native".to_string(),
                amount: Uint128::new(200_000_000u128),
            }],
        )
        .unwrap();

        // swap cw20 -> native
        app.execute_contract(
            creator.sender.clone(),
            cw20_token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: pair_tokens[i].to_string(),
                amount: Uint128::new(200_000_000u128),
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        // Verify the fees are being collected
        let query_protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_tokens[i],
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        for asset in query_protocol_fees_res.fees {
            assert!(asset.amount > Uint128::zero());
            accumulate_fee(&mut assets_collected, &asset);
        }
    }

    // assert the assets collected are the native token + the tokens created
    assert_eq!(assets_collected.len(), TOKEN_AMOUNT + 1);

    // Make sure the fee collector's balance for the assets in which fees were collected is zero
    for (asset_id, _) in assets_collected.clone() {
        if asset_id == "native" {
            let balance_res = app
                .wrap()
                .query_balance(fee_collector_address.clone().to_string(), "native")
                .unwrap();
            assert_eq!(balance_res.amount, Uint128::zero());
        } else {
            let balance_res: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    &asset_id,
                    &cw20::Cw20QueryMsg::Balance {
                        address: fee_collector_address.clone().to_string(),
                    },
                )
                .unwrap();
            assert_eq!(balance_res.balance, Uint128::zero());
        }
    }

    // Collect the fees with pagination
    let mut start_after: Option<[AssetInfo; 2]> = None;
    let mut i = 0;
    // there are 10 pools in this test, and we set the pagination limit to half of that. So we will
    // collect the fees twice using pagination
    while i < 2 {
        let pairs_response: PairsResponse = app
            .wrap()
            .query_wasm_smart(
                &pool_factory_address,
                &pool_network::factory::QueryMsg::Pairs {
                    start_after: start_after.clone(),
                    limit: Some(u32::try_from(TOKEN_AMOUNT / 2).unwrap()),
                },
            )
            .unwrap();

        app.execute_contract(
            creator.sender.clone(),
            fee_collector_address.clone(),
            &CollectFees {
                collect_fees_for: FeesFor::Factory {
                    factory_addr: pool_factory_address.to_string(),
                    factory_type: FactoryType::Pool {
                        start_after: start_after.clone(),
                        limit: Some(u32::try_from(TOKEN_AMOUNT / 2).unwrap()),
                    },
                },
            },
            &[],
        )
        .unwrap();

        start_after = Some(
            pairs_response
                .clone()
                .pairs
                .clone()
                .last()
                .cloned()
                .unwrap()
                .asset_infos,
        );

        i += 1;
    }

    // Make sure the fee collector's balance for the assets in which fees were collected increased,
    // and matches the amount the pool reported to have collected
    for (asset_id, asset) in assets_collected.clone() {
        if asset_id == "native" {
            let balance_res = app
                .wrap()
                .query_balance(fee_collector_address.clone().to_string(), "native")
                .unwrap();
            assert!(balance_res.amount > Uint128::zero());
            assert_eq!(balance_res.amount, asset.amount);
        } else {
            let balance_res: BalanceResponse = app
                .wrap()
                .query_wasm_smart(
                    &asset_id,
                    &cw20::Cw20QueryMsg::Balance {
                        address: fee_collector_address.clone().to_string(),
                    },
                )
                .unwrap();
            assert!(balance_res.balance > Uint128::zero());
            assert_eq!(balance_res.balance, asset.amount);
        }
    }

    // Make sure protocol fees in the pools are zero, as they have been collected
    for pair_token in pair_tokens {
        let protocol_fees_res: ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &pair_token.clone(),
                &pool_network::pair::QueryMsg::ProtocolFees {
                    asset_id: None,
                    all_time: None,
                },
            )
            .unwrap();

        for fee in protocol_fees_res.fees {
            assert_eq!(fee.amount, Uint128::zero());
        }
    }
}

#[test]
fn collect_fees_for_vault() {
    let creator = mock_creator();
    let native_tokens = vec![
        Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(500_000_000u128),
        },
        Coin {
            denom: "ujuno".to_string(),
            amount: Uint128::new(500_000_000u128),
        },
        Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(500_000_000u128),
        },
    ];

    let balances = vec![(creator.clone().sender, native_tokens.clone())];
    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let vault_factory_id = store_vault_factory_code(&mut app);
    let token_id = store_token_code(&mut app);
    let vault_id = store_vault_code(&mut app);
    let dummy_flash_loan_id = store_dummy_flash_loan_contract(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let vault_factory_address = app
        .instantiate_contract(
            vault_factory_id,
            creator.clone().sender,
            &white_whale::vault_network::vault_factory::InstantiateMsg {
                owner: creator.clone().sender.into_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_address.clone().into_string(),
            },
            &[],
            "vault_factory",
            None,
        )
        .unwrap();

    let dummy_flash_loan_address = app
        .instantiate_contract(
            dummy_flash_loan_id,
            creator.clone().sender,
            &crate::tests::dummy_contract::InstantiateMsg {},
            &[],
            "dummy flash-loan",
            None,
        )
        .unwrap();

    // Create few vaults
    let flash_loan_fee = Fee {
        share: Decimal::from_ratio(100u128, 3000u128),
    };
    let mut vaults: Vec<Addr> = Vec::new();
    for coin in native_tokens.clone() {
        let res = app
            .execute_contract(
                creator.clone().sender,
                vault_factory_address.clone(),
                &ExecuteMsg::CreateVault {
                    asset_info: AssetInfo::NativeToken {
                        denom: coin.clone().denom.to_string(),
                    },
                    fees: VaultFee {
                        flash_loan_fee: Fee {
                            share: Decimal::from_ratio(100u128, 3000u128),
                        },
                        protocol_fee: flash_loan_fee.clone(),
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                },
                &[],
            )
            .unwrap();

        let created_vault_addr = res
            .events
            .iter()
            .flat_map(|event| &event.attributes)
            .find(|attribute| attribute.key == "vault_address")
            .unwrap();

        vaults.push(Addr::unchecked(created_vault_addr.clone().value));
    }

    // Deposit coins into vaults
    for (i, coin) in native_tokens.iter().enumerate() {
        app.execute_contract(
            creator.clone().sender.clone(),
            vaults[i].clone(),
            &white_whale::vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(400_000_000u128),
            },
            &[Coin {
                denom: coin.clone().denom,
                amount: Uint128::new(400_000_000u128),
            }],
        )
        .unwrap();

        let transfer = BankMsg::Send {
            to_address: dummy_flash_loan_address.clone().to_string(),
            amount: vec![Coin {
                denom: coin.clone().denom,
                amount: Uint128::new(100_000_000u128),
            }],
        };
        app.execute(creator.clone().sender, transfer.into())
            .unwrap();
    }

    let flash_loan_value = 500_000u128;
    let return_flash_loan_value = 600_000u128;
    let computed_protocol_fees = flash_loan_fee.compute(Uint256::from(flash_loan_value));

    // Verify no fees have been generated via the collector's query
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Factory {
                    factory_addr: vault_factory_address.to_string(),
                    factory_type: FactoryType::Vault {
                        start_after: None,
                        limit: None,
                    },
                },
                all_time: None,
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 3usize);
    for asset in fee_collector_fees_query {
        assert_eq!(asset.amount, Uint128::zero());
    }

    // Perform some flashloans
    for (i, coin) in native_tokens.iter().enumerate() {
        // verify the protocol fees are zero before the flashloan
        let query_protocol_fees_res: white_whale::vault_network::vault::ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &vaults[i],
                &white_whale::vault_network::vault::QueryMsg::ProtocolFees { all_time: false },
            )
            .unwrap();
        assert_eq!(query_protocol_fees_res.fees.amount, Uint128::zero());

        // make a dummy message which transfers desired amount back to vault
        app.execute_contract(
            dummy_flash_loan_address.clone(),
            vaults[i].clone(),
            &white_whale::vault_network::vault::ExecuteMsg::FlashLoan {
                amount: Uint128::new(flash_loan_value),
                msg: to_binary(&BankMsg::Send {
                    to_address: vaults[i].to_string(),
                    // return a higher amount than the flashloan + fees
                    amount: coins(return_flash_loan_value, coin.denom.clone()),
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        // verify the protocol fees where collected after flashloan
        let query_protocol_fees_res: white_whale::vault_network::vault::ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &vaults[i],
                &white_whale::vault_network::vault::QueryMsg::ProtocolFees { all_time: false },
            )
            .unwrap();
        assert!(query_protocol_fees_res.fees.amount > Uint128::zero());
        assert_eq!(
            computed_protocol_fees,
            Uint256::from(query_protocol_fees_res.fees.amount)
        );
    }

    // Collect the fees accrued by the flashloan operations
    app.execute_contract(
        creator.sender,
        fee_collector_address.clone(),
        &CollectFees {
            collect_fees_for: FeesFor::Factory {
                factory_addr: vault_factory_address.to_string(),
                factory_type: FactoryType::Vault {
                    start_after: None,
                    limit: None,
                },
            },
        },
        &[],
    )
    .unwrap();

    // verify the fee collector got the funds
    for native_token in native_tokens {
        let balance_res: Coin = app
            .wrap()
            .query_balance(
                fee_collector_address.clone().to_string(),
                native_token.denom,
            )
            .unwrap();

        assert!(balance_res.amount > Uint128::zero());
        assert_eq!(Uint256::from(balance_res.amount), computed_protocol_fees);
    }

    // verify the protocol fees are zero after collecting the fees from the flashloans
    for vault in vaults.clone() {
        let query_protocol_fees_res: white_whale::vault_network::vault::ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &vault,
                &white_whale::vault_network::vault::QueryMsg::ProtocolFees { all_time: false },
            )
            .unwrap();
        assert_eq!(query_protocol_fees_res.fees.amount, Uint128::zero());
    }

    // Verify fees via the collector's query
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Factory {
                    factory_addr: vault_factory_address.to_string(),
                    factory_type: FactoryType::Vault {
                        start_after: None,
                        limit: None,
                    },
                },
                all_time: None,
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 3usize);
    for asset in fee_collector_fees_query {
        // zero, as it was just collected
        assert_eq!(asset.amount, Uint128::zero());
    }

    // Verify all time fees via the collector's query for a single vault
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address,
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Contracts {
                    contracts: vec![Contract {
                        address: vaults[0].to_string(),
                        contract_type: ContractType::Vault {},
                    }],
                },
                all_time: Some(true),
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 1usize);
    let expected_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: "uatom".to_string(),
        },
        amount: Uint128::new(16666),
    };
    assert_eq!(fee_collector_fees_query[0], expected_asset);
}

#[test]
fn aggregate_fees_for_vault() {
    let creator = mock_creator();
    let native_tokens = vec![
        Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(900_000_000u128),
        },
        Coin {
            denom: "ujuno".to_string(),
            amount: Uint128::new(900_000_000u128),
        },
    ];

    let balances = vec![(creator.clone().sender, native_tokens.clone())];
    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let vault_factory_id = store_vault_factory_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);
    let vault_id = store_vault_code(&mut app);
    let dummy_flash_loan_id = store_dummy_flash_loan_contract(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let vault_factory_address = app
        .instantiate_contract(
            vault_factory_id,
            creator.clone().sender,
            &white_whale::vault_network::vault_factory::InstantiateMsg {
                owner: creator.clone().sender.into_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_address.clone().into_string(),
            },
            &[],
            "vault_factory",
            None,
        )
        .unwrap();

    let dummy_flash_loan_address = app
        .instantiate_contract(
            dummy_flash_loan_id,
            creator.clone().sender,
            &crate::tests::dummy_contract::InstantiateMsg {},
            &[],
            "dummy flash-loan",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    // add native token to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "uatom".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "uatom".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "ujuno".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "ujuno".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: None,
            pool_factory: None,
            vault_factory: None,
        },
        &[],
    )
    .unwrap();

    // Create few vaults
    let flash_loan_fee = Fee {
        share: Decimal::from_ratio(100u128, 3000u128),
    };
    let mut vaults: Vec<Addr> = Vec::new();
    for coin in native_tokens.clone() {
        let res = app
            .execute_contract(
                creator.clone().sender,
                vault_factory_address.clone(),
                &ExecuteMsg::CreateVault {
                    asset_info: AssetInfo::NativeToken {
                        denom: coin.clone().denom.to_string(),
                    },
                    fees: VaultFee {
                        flash_loan_fee: Fee {
                            share: Decimal::from_ratio(100u128, 3000u128),
                        },
                        protocol_fee: flash_loan_fee.clone(),
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                },
                &[],
            )
            .unwrap();

        let created_vault_addr = res
            .events
            .iter()
            .flat_map(|event| &event.attributes)
            .find(|attribute| attribute.key == "vault_address")
            .unwrap();

        vaults.push(Addr::unchecked(created_vault_addr.clone().value));
    }

    // Deposit coins into vaults
    for (i, coin) in native_tokens.iter().enumerate() {
        app.execute_contract(
            creator.clone().sender.clone(),
            vaults[i].clone(),
            &white_whale::vault_network::vault::ExecuteMsg::Deposit {
                amount: Uint128::new(400_000_000u128),
            },
            &[Coin {
                denom: coin.clone().denom,
                amount: Uint128::new(400_000_000u128),
            }],
        )
        .unwrap();

        let transfer = BankMsg::Send {
            to_address: dummy_flash_loan_address.clone().to_string(),
            amount: vec![Coin {
                denom: coin.clone().denom,
                amount: Uint128::new(100_000_000u128),
            }],
        };
        app.execute(creator.clone().sender, transfer.into())
            .unwrap();
    }

    let flash_loan_value = 500_000u128;
    let return_flash_loan_value = 600_000u128;
    let computed_protocol_fees = flash_loan_fee.compute(Uint256::from(flash_loan_value));

    // Verify no fees have been generated via the collector's query
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Factory {
                    factory_addr: vault_factory_address.to_string(),
                    factory_type: FactoryType::Vault {
                        start_after: None,
                        limit: None,
                    },
                },
                all_time: None,
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 2usize);
    for asset in fee_collector_fees_query {
        assert_eq!(asset.amount, Uint128::zero());
    }

    // Perform some flashloans
    for (i, coin) in native_tokens.iter().enumerate() {
        // verify the protocol fees are zero before the flashloan
        let query_protocol_fees_res: white_whale::vault_network::vault::ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &vaults[i],
                &white_whale::vault_network::vault::QueryMsg::ProtocolFees { all_time: false },
            )
            .unwrap();
        assert_eq!(query_protocol_fees_res.fees.amount, Uint128::zero());

        // make a dummy message which transfers desired amount back to vault
        app.execute_contract(
            dummy_flash_loan_address.clone(),
            vaults[i].clone(),
            &white_whale::vault_network::vault::ExecuteMsg::FlashLoan {
                amount: Uint128::new(flash_loan_value),
                msg: to_binary(&BankMsg::Send {
                    to_address: vaults[i].to_string(),
                    // return a higher amount than the flashloan + fees
                    amount: coins(return_flash_loan_value, coin.denom.clone()),
                })
                .unwrap(),
            },
            &[],
        )
        .unwrap();

        // verify the protocol fees where collected after flashloan
        let query_protocol_fees_res: white_whale::vault_network::vault::ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &vaults[i],
                &white_whale::vault_network::vault::QueryMsg::ProtocolFees { all_time: false },
            )
            .unwrap();
        assert!(query_protocol_fees_res.fees.amount > Uint128::zero());
        assert_eq!(
            computed_protocol_fees,
            Uint256::from(query_protocol_fees_res.fees.amount)
        );
    }

    // Collect the fees accrued by the flashloan operations
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &CollectFees {
            collect_fees_for: FeesFor::Factory {
                factory_addr: vault_factory_address.to_string(),
                factory_type: FactoryType::Vault {
                    start_after: None,
                    limit: None,
                },
            },
        },
        &[],
    )
    .unwrap();

    // verify the fee collector got the funds
    for native_token in native_tokens.clone() {
        let balance_res: Coin = app
            .wrap()
            .query_balance(
                fee_collector_address.clone().to_string(),
                native_token.denom,
            )
            .unwrap();

        assert!(balance_res.amount > Uint128::zero());
        assert_eq!(Uint256::from(balance_res.amount), computed_protocol_fees);
    }

    // verify the protocol fees are zero after collecting the fees from the flashloans
    for vault in vaults.clone() {
        let query_protocol_fees_res: white_whale::vault_network::vault::ProtocolFeesResponse = app
            .wrap()
            .query_wasm_smart(
                &vault,
                &white_whale::vault_network::vault::QueryMsg::ProtocolFees { all_time: false },
            )
            .unwrap();
        assert_eq!(query_protocol_fees_res.fees.amount, Uint128::zero());
    }

    // Verify fees via the collector's query
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Factory {
                    factory_addr: vault_factory_address.to_string(),
                    factory_type: FactoryType::Vault {
                        start_after: None,
                        limit: None,
                    },
                },
                all_time: None,
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 2usize);
    for asset in fee_collector_fees_query {
        // zero, as it was just collected
        assert_eq!(asset.amount, Uint128::zero());
    }

    // Verify all time fees via the collector's query for a single vault
    let fee_collector_fees_query: Vec<Asset> = app
        .wrap()
        .query_wasm_smart(
            fee_collector_address.clone(),
            &QueryMsg::Fees {
                query_fees_for: FeesFor::Contracts {
                    contracts: vec![Contract {
                        address: vaults[0].to_string(),
                        contract_type: ContractType::Vault {},
                    }],
                },
                all_time: Some(true),
            },
        )
        .unwrap();

    assert_eq!(fee_collector_fees_query.len(), 1usize);
    let expected_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: "uatom".to_string(),
        },
        amount: Uint128::new(16666),
    };
    assert_eq!(fee_collector_fees_query[0], expected_asset);

    // Try aggregating the fees without adding a swap route, this should do nothing as assets without
    // a swap route are skipped and not aggregated

    let ask_asset = AssetInfo::NativeToken {
        denom: "uatom".to_string(),
    };
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &AggregateFees {
            asset_info: ask_asset.clone(),
            aggregate_fees_for: FeesFor::Factory {
                factory_addr: vault_factory_address.to_string(),
                factory_type: FactoryType::Vault {
                    start_after: None,
                    limit: None,
                },
            },
        },
        &[],
    )
    .unwrap();

    // verify the fees collected were not aggregated
    let mut ask_asset_original_balance = Uint128::zero();
    for native_token in native_tokens.clone() {
        let balance_res: Coin = app
            .wrap()
            .query_balance(
                fee_collector_address.clone().to_string(),
                native_token.clone().denom,
            )
            .unwrap();

        assert!(balance_res.amount > Uint128::zero());
        if native_token.clone().denom == ask_asset.clone().to_string() {
            ask_asset_original_balance = balance_res.amount;
        }
    }

    // Create a pool so that we can swap those assets
    let res = app
        .execute_contract(
            creator.sender.clone(),
            pool_factory_address,
            &CreatePair {
                asset_infos: [
                    AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "ujuno".to_string(),
                    },
                ],
                pool_fees: PoolFee {
                    protocol_fee: Fee {
                        share: Decimal::percent(5u64),
                    },
                    swap_fee: Fee {
                        share: Decimal::percent(7u64),
                    },
                    burn_fee: Fee {
                        share: Decimal::zero(),
                    },
                },
                pair_type: PairType::ConstantProduct,
                token_factory_lp: false,
            },
            &[],
        )
        .unwrap();

    let pool_address = Addr::unchecked(
        res.events
            .last()
            .unwrap()
            .attributes
            .clone()
            .get(1)
            .unwrap()
            .clone()
            .value,
    );

    // provide liquidity to the pool
    app.execute_contract(
        creator.sender.clone(),
        pool_address,
        &pool_network::pair::ExecuteMsg::ProvideLiquidity {
            assets: [
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uatom".to_string(),
                    },
                    amount: Uint128::new(50_000u128),
                },
                Asset {
                    info: AssetInfo::NativeToken {
                        denom: "ujuno".to_string(),
                    },
                    amount: Uint128::new(50_000u128),
                },
            ],
            slippage_tolerance: None,
            receiver: None,
        },
        &[
            Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(50_000u128),
            },
            Coin {
                denom: "ujuno".to_string(),
                amount: Uint128::new(50_000u128),
            },
        ],
    )
    .unwrap();

    // Add the swap route
    let swap_operations = vec![SwapOperation::TerraSwap {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "ujuno".to_string(),
        },
        ask_asset_info: ask_asset.clone(),
    }];

    let swap_routes: Vec<SwapRoute> = vec![SwapRoute {
        offer_asset_info: AssetInfo::NativeToken {
            denom: "ujuno".to_string(),
        },
        ask_asset_info: ask_asset.clone(),
        swap_operations,
    }];

    app.execute_contract(
        creator.sender.clone(),
        pool_router_address,
        &pool_network::router::ExecuteMsg::AddSwapRoutes { swap_routes },
        &[],
    )
    .unwrap();

    // Aggregate fees
    app.execute_contract(
        creator.sender,
        fee_collector_address.clone(),
        &AggregateFees {
            asset_info: ask_asset.clone(),
            aggregate_fees_for: FeesFor::Factory {
                factory_addr: vault_factory_address.to_string(),
                factory_type: FactoryType::Vault {
                    start_after: None,
                    limit: None,
                },
            },
        },
        &[],
    )
    .unwrap();

    // verify the fees collected were aggregated
    for native_token in native_tokens {
        let balance_res: Coin = app
            .wrap()
            .query_balance(
                fee_collector_address.clone().to_string(),
                native_token.clone().denom,
            )
            .unwrap();

        if native_token.denom == ask_asset.to_string() {
            assert!(balance_res.amount > ask_asset_original_balance);
        } else {
            assert_eq!(balance_res.amount, Uint128::zero());
        }
    }
}

fn accumulate_fee(assets_collected: &mut HashMap<String, Asset>, asset: &Asset) {
    let asset_id = asset.clone().get_id();
    if let Some(collected) = assets_collected.get(asset_id.as_str()) {
        assets_collected.insert(
            asset_id.clone(),
            Asset {
                info: asset.info.clone(),
                amount: collected.amount.checked_add(asset.amount).unwrap(),
            },
        );
    } else {
        assets_collected.insert(
            asset_id,
            Asset {
                info: asset.info.clone(),
                amount: asset.amount,
            },
        );
    }
}

#[test]
fn accumulate_fee_works() {
    let mut assets_collected: HashMap<String, Asset> = HashMap::new();
    let asset_fee_1 = Asset {
        info: AssetInfo::Token {
            contract_addr: "asset1".to_string(),
        },
        amount: Uint128::new(100u128),
    };
    let asset_fee_2 = Asset {
        info: AssetInfo::Token {
            contract_addr: "asset1".to_string(),
        },
        amount: Uint128::new(200u128),
    };
    let asset_fee_3 = Asset {
        info: AssetInfo::NativeToken {
            denom: "native".to_string(),
        },
        amount: Uint128::new(50u128),
    };

    accumulate_fee(&mut assets_collected, &asset_fee_1);
    accumulate_fee(&mut assets_collected, &asset_fee_2);
    accumulate_fee(&mut assets_collected, &asset_fee_3);

    assert_eq!(assets_collected.len(), 2);
    for (id, asset) in assets_collected {
        if id == "asset1" {
            assert_eq!(asset.amount, Uint128::new(300u128));
        } else if id == "native" {
            assert_eq!(asset.amount, Uint128::new(50u128));
        }
    }
}

#[test]
fn collect_and_distribute_fees_successfully() {
    let creator = mock_creator();
    let balances = vec![(
        creator.clone().sender,
        vec![
            coin(1_000_000_000, "usdc"),
            coin(1_000_000_000, "uwhale"),
            coin(1_000_000_000, "ampWHALE"),
            coin(1_000_000_000, "bWHALE"),
        ],
    )];

    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let fee_distributor_id = store_fee_distributor_code(&mut app);
    let whale_lair_id = store_whale_lair_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);
    let vault_factory_id = store_vault_factory_code(&mut app);
    let vault_id = store_vault_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let vault_factory_address = app
        .instantiate_contract(
            vault_factory_id,
            creator.clone().sender,
            &vault_network::vault_factory::InstantiateMsg {
                owner: creator.clone().sender.to_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let whale_lair_address = app
        .instantiate_contract(
            whale_lair_id,
            creator.clone().sender,
            &white_whale::whale_lair::InstantiateMsg {
                unbonding_period: Uint64::new(1_000_000_000_000u64),
                growth_rate: Decimal::one(),
                bonding_assets: vec![
                    AssetInfo::NativeToken {
                        denom: "ampWHALE".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "bWHALE".to_string(),
                    },
                ],
            },
            &[],
            "whale_lair",
            None,
        )
        .unwrap();

    let fee_distributor_address = app
        .instantiate_contract(
            fee_distributor_id,
            creator.clone().sender,
            &white_whale::fee_distributor::InstantiateMsg {
                bonding_contract_addr: whale_lair_address.clone().to_string(),
                fee_collector_addr: fee_collector_address.clone().to_string(),
                grace_period: Uint64::new(2),
                epoch_config: EpochConfig {
                    duration: Uint64::new(86_400_000_000_000u64), // a day
                    genesis_epoch: Uint64::new(1678802400_000000000u64), // March 14, 2023 2:00:00 PM
                },
                distribution_asset: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
            &[],
            "fee_distributor",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: Some(fee_distributor_address.to_string()),
            pool_factory: Some(pool_factory_address.to_string()),
            vault_factory: Some(vault_factory_address.to_string()),
        },
        &[],
    )
    .unwrap();

    // add native tokens to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "uwhale".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "usdc".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "ampWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "bWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "bWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    // Create few pools
    let native_tokens: Vec<&str> = vec!["usdc", "ampWHALE", "bWHALE"];
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for native_token in native_tokens.clone() {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        let pair_address = Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        );
        pair_tokens.push(pair_address);
    }

    // Provide liquidity into pools
    for (i, native_token) in native_tokens.clone().iter().enumerate() {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(500_000u128),
                },
                Coin {
                    denom: native_token.clone().to_string(),
                    amount: Uint128::new(500_000u128),
                },
            ],
        )
        .unwrap();
    }

    // Perform some swaps
    for (i, native_token) in native_tokens.clone().iter().enumerate() {
        // whale -> native
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "uwhale".to_string(),
                    },
                    amount: Uint128::new(200_000_000u128),
                },
                belief_price: None,
                max_spread: None,
                to: None,
            },
            &[Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::new(200_000_000u128),
            }],
        )
        .unwrap();

        // native -> whale
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::Swap {
                offer_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: native_token.clone().to_string(),
                    },
                    amount: Uint128::new(200_000_000u128),
                },
                belief_price: None,
                max_spread: None,
                to: None,
            },
            &[Coin {
                denom: native_token.clone().to_string(),
                amount: Uint128::new(200_000_000u128),
            }],
        )
        .unwrap();
    }

    // query current epoch from fee distributor, assert that is equal to the default epoch
    let fee_distributor_current_epoch_query: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();
    // it means no epoch has been created yet
    assert_eq!(fee_distributor_current_epoch_query.epoch, Epoch::default());

    app.set_block(BlockInfo {
        time: Timestamp::from_nanos(1678802400_000000000u64),
        ..app.block_info()
    });

    // Create new epoch, which triggers fee collection, aggregation and distribution
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // check that a new epoch was created
    let fee_distributor_current_epoch_query: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();
    assert_eq!(fee_distributor_current_epoch_query.epoch.id, Uint64::one());
    assert!(!fee_distributor_current_epoch_query.epoch.total.is_empty());
}


#[test]
fn collect_and_dist_fees_where_one_bonder_is_increasing_weight() {
    let creator = mock_creator();
    let balances = vec![
        (
            creator.clone().sender,
            vec![
                coin(1_000_000_000, "usdc"),
                coin(1_000_000_000, "uwhale"),
                coin(1_000_000_000, "ampWHALE"),
                coin(1_000_000_000, "bWHALE"),
            ],
        ),
        (
            Addr::unchecked("other"),
            vec![
                coin(1_000_000_000, "usdc"),
                coin(1_000_000_000, "uwhale"),
                coin(1_000_000_000, "ampWHALE"),
                coin(1_000_000_000, "bWHALE"),
            ],
        ),
    ];

    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let fee_distributor_id = store_fee_distributor_code(&mut app);
    let whale_lair_id = store_whale_lair_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);
    let vault_factory_id = store_vault_factory_code(&mut app);
    let vault_id = store_vault_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let vault_factory_address = app
        .instantiate_contract(
            vault_factory_id,
            creator.clone().sender,
            &vault_network::vault_factory::InstantiateMsg {
                owner: creator.clone().sender.to_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let whale_lair_address = app
        .instantiate_contract(
            whale_lair_id,
            creator.clone().sender,
            &white_whale::whale_lair::InstantiateMsg {
                unbonding_period: Uint64::new(1_000_000_000_000u64),
                growth_rate: Decimal::one(),
                bonding_assets: vec![
                    AssetInfo::NativeToken {
                        denom: "ampWHALE".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "bWHALE".to_string(),
                    },
                ],
            },
            &[],
            "whale_lair",
            None,
        )
        .unwrap();

    let fee_distributor_address = app
        .instantiate_contract(
            fee_distributor_id,
            creator.clone().sender,
            &white_whale::fee_distributor::InstantiateMsg {
                bonding_contract_addr: whale_lair_address.clone().to_string(),
                fee_collector_addr: fee_collector_address.clone().to_string(),
                grace_period: Uint64::new(1),
                epoch_config: EpochConfig {
                    duration: Uint64::new(86_400_000_000_000u64), // a day
                    genesis_epoch: Uint64::new(1678802400_000000000u64), // March 14, 2023 2:00:00 PM
                },
                distribution_asset: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
            &[],
            "fee_distributor",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: Some(fee_distributor_address.to_string()),
            pool_factory: Some(pool_factory_address.to_string()),
            vault_factory: Some(vault_factory_address.to_string()),
        },
        &[],
    )
    .unwrap();

    // add native tokens to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "uwhale".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "usdc".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "ampWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "bWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "bWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    // Create few pools
    let native_tokens: Vec<&str> = vec!["usdc", "ampWHALE", "bWHALE"];
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for native_token in native_tokens.clone() {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        let pair_address = Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        );
        pair_tokens.push(pair_address);
    }

    // Provide liquidity into pools
    for (i, native_token) in native_tokens.clone().iter().enumerate() {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(500_000u128),
                },
                Coin {
                    denom: native_token.clone().to_string(),
                    amount: Uint128::new(500_000u128),
                },
            ],
        )
        .unwrap();
    }

    // bond some tokens - 1k each 
    app.execute_contract(
        creator.sender.clone(),
        whale_lair_address.clone(),
        &white_whale::whale_lair::ExecuteMsg::Bond {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(1_000u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked("other").clone(),
        whale_lair_address.clone(),
        &white_whale::whale_lair::ExecuteMsg::Bond {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(1_000u128),
        }],
    )
    .unwrap();

    // Create EPOCH 1 with 100 whale 
    // whale -> native
    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(2_010u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(2_010u128),
        }],
    )
    .unwrap();

    // advance the time to one day after the first epoch was created
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678888800_000000000u64),
        chain_id: "".to_string(),
    });

    // Create new epoch, which triggers fee collection, aggregation and distribution
    // Verify epoch 1 
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // check that a new epoch was created
    let expiring_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();
    assert_eq!(expiring_epoch_res.epoch.id, Uint64::one());
    assert_eq!(
        expiring_epoch_res.epoch.available,
        expiring_epoch_res.epoch.total
    );
    assert!(expiring_epoch_res.epoch.claimed.is_empty());
    // Verify  expiring_epoch_res.epoch.available, has 100 whale as an Asset
    assert_eq!(
        expiring_epoch_res.epoch.available,
        vec![Asset {
            info: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
            amount: Uint128::new(100u128),
        }]
    );

    // When creating the second epoch, the first one will be expiring since the grace_period was set to 1.
    // Make sure the available tokens on the expiring epoch are transferred to the second one.
    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(2_050u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(2_050u128),
        }],
    )
    .unwrap();

    // advance the time to one day after the first epoch was created
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678888800_000000000u64),
        chain_id: "".to_string(),
    });

    // Bond 500 more with user 1 
    app.execute_contract(
        creator.sender.clone(),
        whale_lair_address.clone(),
        &white_whale::whale_lair::ExecuteMsg::Bond {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(500u128),
            },
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(500u128),
        }],
    ).unwrap();

    // Create new epoch, which triggers fee collection, aggregation and distribution
    // Create EPOCH 2
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // check that the second epoch was created
    let new_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();

    assert_eq!(new_epoch_res.epoch.id, Uint64::new(2u64));
    assert_eq!(new_epoch_res.epoch.available, new_epoch_res.epoch.total);
    assert!(new_epoch_res.epoch.claimed.is_empty());

    // check that the available assets for the expired epoch are zero/empty
    let expired_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::Epoch { id: Uint64::one() },
        )
        .unwrap();
    assert!(expired_epoch_res.epoch.available.is_empty());

    // since the fees collected for the second epoch were the same for the first, the available
    // assets for the second epoch should be twice the amount of the first

    // iterate the new_epoch_res.epoch.available and add up the amounts for each asset
    let mut total_amount_new_epoch = Uint128::zero();
    for asset in new_epoch_res.epoch.available {
        total_amount_new_epoch += asset.amount;
    }
    println!("total_amount_new_epoch: {}", total_amount_new_epoch);
    let mut total_amount_expired = Uint128::zero();
    //checking against total since total and available where the same, but available is empty now
    for asset in expired_epoch_res.epoch.total {
        total_amount_expired += asset.amount;
    }
    println!("total_amount_expired: {}", total_amount_expired);
    assert!(total_amount_new_epoch - total_amount_expired > Uint128::zero());

    // Bond 500 more with user 1 
    app.execute_contract(
        creator.sender.clone(),
        whale_lair_address.clone(),
        &white_whale::whale_lair::ExecuteMsg::Bond {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(500u128),
            },
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(500u128),
        }],
    ).unwrap();

     // Make sure the available tokens on the expiring epoch are transferred to the second one.
     app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(2_050u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(2_050u128),
        }],
    )
    .unwrap();

    // Now we can advance time, create a third epoch and check that the fees collected are
    // distributed to the users
    // advance the time to one day after the second epoch was created
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(3357777600_000000000u64),
        chain_id: "".to_string(),
    });
   
    // Create new epoch, which triggers fee collection, aggregation and distribution
    // Create EPOCH 3

    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // check that the third epoch was created
    let new_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();

    assert_eq!(new_epoch_res.epoch.id, Uint64::new(3u64));
    assert_eq!(new_epoch_res.epoch.available, new_epoch_res.epoch.total);
    assert!(new_epoch_res.epoch.claimed.is_empty());

    // check that the available assets for the expired epoch are zero/empty
    let expired_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::Epoch { id: Uint64::from(2u64) },
        )
        .unwrap();
    assert!(expired_epoch_res.epoch.available.is_empty());

    // Advance time one more time 
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(503666400_000000000u64),
        chain_id: "".to_string(),
    });
    // 335777600_000000000u64 is 2 days what is 335777600_000000000u64 / 2 * 3
    // 503666400_000000000u64 is 3 days

    // We should have about triple the amount of fees collected in the third epoch
    // compared to the first 
    // iterate the new_epoch_res.epoch.available and add up the amounts for each asset
    let mut total_amount_new_epoch = Uint128::zero();

    for asset in new_epoch_res.epoch.available {
        total_amount_new_epoch += asset.amount;
    }
    println!("total_amount_new_epoch: {}", total_amount_new_epoch);
    let mut total_amount_expired = Uint128::zero();
    //checking against total since total and available where the same, but available is empty now
    for asset in expired_epoch_res.epoch.total {
        total_amount_expired += asset.amount;
    }
    println!("total_amount_expired: {}", total_amount_expired);
    assert!(total_amount_new_epoch - total_amount_expired > Uint128::zero());


    // Lets do some claims 

    // claim some rewards
    let uwhale_balance_before_claiming = app
        .wrap()
        .query_balance(Addr::unchecked("other"), "uwhale")
        .unwrap()
        .amount;
    // Claim 1
    app.execute_contract(
        Addr::unchecked("other"),
        fee_distributor_address.clone(),
        &white_whale::fee_distributor::ExecuteMsg::Claim {},
        &[],
    )
    .unwrap();

    let uwhale_balance_after_claiming = app
    .wrap()
    .query_balance(Addr::unchecked("other"), "uwhale")
    .unwrap()
    .amount;

    let whale_received = uwhale_balance_after_claiming - uwhale_balance_before_claiming;
    println!("whale_received: {}", whale_received);
    assert!(whale_received > Uint128::zero());
    // assert_eq!(whale_received, Uint128::new(190u128));
    // For Claim 1 we should have 190 uwhale which is assuming 100 whale per epoch and they have 50% of the first 60% of the second and 80% of the third = 190
    // 100 + 30 + 60 = 190


    // Claim 2 
    // claim some rewards
    let uwhale_balance_before_claiming = app
        .wrap()
        .query_balance(creator.sender.clone(), "uwhale")
        .unwrap()
        .amount;

    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &white_whale::fee_distributor::ExecuteMsg::Claim {},
        &[],
    )
    .unwrap();

    let uwhale_balance_after_claiming = app
        .wrap()
        .query_balance(creator.sender.clone(), "uwhale")
        .unwrap()
        .amount;

    let other_whale_received = uwhale_balance_after_claiming - uwhale_balance_before_claiming;
    println!("whale_received: {}", whale_received);
    // assert_eq!(whale_received, Uint128::new(190u128));
    // For Claim 1 we should have 190 uwhale which is assuming 100 whale per epoch and they have 50% of the first 60% of the second and 80% of the third = 190
    // 100 + 30 + 60 = 190
    // Verify the amounts statically 
    assert_eq!(whale_received, other_whale_received);
    // Above should be be equal though 
    // whale_received should be 190 and other_whale_received should be 110 making 300 if weights and new bonds are respected 
}


#[test]
fn collect_and_distribute_fees_with_expiring_epoch_successfully() {
    let creator = mock_creator();
    let balances = vec![
        (
            creator.clone().sender,
            vec![
                coin(1_000_000_000, "usdc"),
                coin(1_000_000_000, "uwhale"),
                coin(1_000_000_000, "ampWHALE"),
                coin(1_000_000_000, "bWHALE"),
            ],
        ),
        (
            Addr::unchecked("other"),
            vec![
                coin(1_000_000_000, "usdc"),
                coin(1_000_000_000, "uwhale"),
                coin(1_000_000_000, "ampWHALE"),
                coin(1_000_000_000, "bWHALE"),
            ],
        ),
    ];

    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let fee_distributor_id = store_fee_distributor_code(&mut app);
    let whale_lair_id = store_whale_lair_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);
    let vault_factory_id = store_vault_factory_code(&mut app);
    let vault_id = store_vault_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let vault_factory_address = app
        .instantiate_contract(
            vault_factory_id,
            creator.clone().sender,
            &vault_network::vault_factory::InstantiateMsg {
                owner: creator.clone().sender.to_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let whale_lair_address = app
        .instantiate_contract(
            whale_lair_id,
            creator.clone().sender,
            &white_whale::whale_lair::InstantiateMsg {
                unbonding_period: Uint64::new(1_000_000_000_000u64),
                growth_rate: Decimal::one(),
                bonding_assets: vec![
                    AssetInfo::NativeToken {
                        denom: "ampWHALE".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "bWHALE".to_string(),
                    },
                ],
            },
            &[],
            "whale_lair",
            None,
        )
        .unwrap();

    let fee_distributor_address = app
        .instantiate_contract(
            fee_distributor_id,
            creator.clone().sender,
            &white_whale::fee_distributor::InstantiateMsg {
                bonding_contract_addr: whale_lair_address.clone().to_string(),
                fee_collector_addr: fee_collector_address.clone().to_string(),
                grace_period: Uint64::new(1),
                epoch_config: EpochConfig {
                    duration: Uint64::new(86_400_000_000_000u64), // a day
                    genesis_epoch: Uint64::new(1678802400_000000000u64), // March 14, 2023 2:00:00 PM
                },
                distribution_asset: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
            &[],
            "fee_distributor",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: Some(fee_distributor_address.to_string()),
            pool_factory: Some(pool_factory_address.to_string()),
            vault_factory: Some(vault_factory_address.to_string()),
        },
        &[],
    )
    .unwrap();

    // add native tokens to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "uwhale".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "usdc".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "ampWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "bWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "bWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    // Create few pools
    let native_tokens: Vec<&str> = vec!["usdc", "ampWHALE", "bWHALE"];
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for native_token in native_tokens.clone() {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        let pair_address = Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        );
        pair_tokens.push(pair_address);
    }

    // Provide liquidity into pools
    for (i, native_token) in native_tokens.clone().iter().enumerate() {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(500_000u128),
                },
                Coin {
                    denom: native_token.clone().to_string(),
                    amount: Uint128::new(500_000u128),
                },
            ],
        )
        .unwrap();
    }

    // bond some tokens
    app.execute_contract(
        creator.sender.clone(),
        whale_lair_address.clone(),
        &white_whale::whale_lair::ExecuteMsg::Bond {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
                amount: Uint128::new(300_000_000u128),
            },
        },
        &[Coin {
            denom: "bWHALE".to_string(),
            amount: Uint128::new(300_000_000u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked("other").clone(),
        whale_lair_address.clone(),
        &white_whale::whale_lair::ExecuteMsg::Bond {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                amount: Uint128::new(100_000_000u128),
            },
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(100_000_000u128),
        }],
    )
    .unwrap();

    // add epochs to the fee distributor.

    // whale -> native
    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(200_000_000u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(200_000_000u128),
        }],
    )
    .unwrap();

    // advance the time to one day after the first epoch was created
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678888800_000000000u64),
        chain_id: "".to_string(),
    });

    // Create new epoch, which triggers fee collection, aggregation and distribution
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // check that a new epoch was created
    let expiring_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();
    assert_eq!(expiring_epoch_res.epoch.id, Uint64::one());
    assert_eq!(
        expiring_epoch_res.epoch.available,
        expiring_epoch_res.epoch.total
    );
    assert!(expiring_epoch_res.epoch.claimed.is_empty());

    // When creating the second epoch, the first one will be expiring since the grace_period was set to 1.
    // Make sure the available tokens on the expiring epoch are transferred to the second one.
    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(200_000_000u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(200_000_000u128),
        }],
    )
    .unwrap();

    // advance the time to one day after the first epoch was created
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678888800_000000000u64),
        chain_id: "".to_string(),
    });

    // Create new epoch, which triggers fee collection, aggregation and distribution
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // check that the second epoch was created
    let new_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();

    assert_eq!(new_epoch_res.epoch.id, Uint64::new(2u64));
    assert_eq!(new_epoch_res.epoch.available, new_epoch_res.epoch.total);
    assert!(new_epoch_res.epoch.claimed.is_empty());

    // check that the available assets for the expired epoch are zero/empty
    let expired_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::Epoch { id: Uint64::one() },
        )
        .unwrap();
    assert!(expired_epoch_res.epoch.available.is_empty());

    // since the fees collected for the second epoch were the same for the first, the available
    // assets for the second epoch should be twice the amount of the first

    // iterate the new_epoch_res.epoch.available and add up the amounts for each asset
    let mut total_amount_new_epoch = Uint128::zero();
    for asset in new_epoch_res.epoch.available {
        total_amount_new_epoch += asset.amount;
    }
    println!("total_amount_new_epoch: {}", total_amount_new_epoch);
    let mut total_amount_expired = Uint128::zero();
    //checking against total since total and available where the same, but available is empty now
    for asset in expired_epoch_res.epoch.total {
        total_amount_expired += asset.amount;
    }
    println!("total_amount_expired: {}", total_amount_expired);
    assert!(total_amount_new_epoch - total_amount_expired > Uint128::zero());

    // claim some rewards
    let uwhale_balance_before_claiming = app
        .wrap()
        .query_balance(creator.sender.clone(), "uwhale")
        .unwrap()
        .amount;

    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &white_whale::fee_distributor::ExecuteMsg::Claim {},
        &[],
    )
    .unwrap();

    let uwhale_balance_after_claiming = app
        .wrap()
        .query_balance(creator.sender.clone(), "uwhale")
        .unwrap()
        .amount;

    assert!(uwhale_balance_after_claiming > uwhale_balance_before_claiming);

    // try to claim again, it should err cause there is nothing to claim
    let err = app
        .execute_contract(
            creator.sender.clone(),
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::ExecuteMsg::Claim {},
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<fee_distributor::ContractError>().unwrap(),
        fee_distributor::ContractError::NothingToClaim {}
    );

    // query the epoch to see if the claimed amount was updated
    let current_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();

    let reward = uwhale_balance_after_claiming - uwhale_balance_before_claiming;

    assert!(current_epoch_res.epoch.total[0].amount > current_epoch_res.epoch.available[0].amount);
    assert_eq!(current_epoch_res.epoch.claimed[0].amount, reward);
    assert_eq!(
        current_epoch_res.epoch.available[0].amount,
        current_epoch_res.epoch.total[0].amount - reward
    );

    app.execute_contract(
        Addr::unchecked("other"),
        fee_distributor_address.clone(),
        &white_whale::fee_distributor::ExecuteMsg::Claim {},
        &[],
    )
    .unwrap();

    let current_epoch_res: EpochResponse = app
        .wrap()
        .query_wasm_smart(
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        )
        .unwrap();
    println!("{:?}", current_epoch_res);
    // all should be claimed by now since both stakers claimed their share
    assert!(current_epoch_res.epoch.available[0].amount <= Uint128::one());
    assert!(
        current_epoch_res.epoch.total[0]
            .amount
            .abs_diff(current_epoch_res.epoch.claimed[0].amount)
            <= Uint128::one()
    );
}

#[test]
fn create_epoch_unsuccessfully() {
    let creator = mock_creator();
    let balances = vec![(
        creator.clone().sender,
        vec![
            coin(1_000_000_000, "usdc"),
            coin(1_000_000_000, "uwhale"),
            coin(1_000_000_000, "ampWHALE"),
            coin(1_000_000_000, "bWHALE"),
        ],
    )];

    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let fee_distributor_id = store_fee_distributor_code(&mut app);
    let whale_lair_id = store_whale_lair_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);
    let vault_factory_id = store_vault_factory_code(&mut app);
    let vault_id = store_vault_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let vault_factory_address = app
        .instantiate_contract(
            vault_factory_id,
            creator.clone().sender,
            &vault_network::vault_factory::InstantiateMsg {
                owner: creator.clone().sender.to_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let whale_lair_address = app
        .instantiate_contract(
            whale_lair_id,
            creator.clone().sender,
            &white_whale::whale_lair::InstantiateMsg {
                unbonding_period: Uint64::new(1_000_000_000_000u64),
                growth_rate: Decimal::one(),
                bonding_assets: vec![
                    AssetInfo::NativeToken {
                        denom: "ampWHALE".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "bWHALE".to_string(),
                    },
                ],
            },
            &[],
            "whale_lair",
            None,
        )
        .unwrap();

    let fee_distributor_address = app
        .instantiate_contract(
            fee_distributor_id,
            creator.clone().sender,
            &white_whale::fee_distributor::InstantiateMsg {
                bonding_contract_addr: whale_lair_address.clone().to_string(),
                fee_collector_addr: fee_collector_address.clone().to_string(),
                grace_period: Uint64::new(1),
                epoch_config: EpochConfig {
                    duration: Uint64::new(86_400_000_000_000u64), // a day
                    genesis_epoch: Uint64::new(1678802400_000000000u64), // March 14, 2023 2:00:00 PM
                },
                distribution_asset: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
            &[],
            "fee_distributor",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: Some(fee_distributor_address.to_string()),
            pool_factory: Some(pool_factory_address.to_string()),
            vault_factory: Some(vault_factory_address.to_string()),
        },
        &[],
    )
    .unwrap();

    // add native tokens to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "uwhale".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "usdc".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "ampWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "bWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "bWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    // Create few pools
    let native_tokens: Vec<&str> = vec!["usdc", "ampWHALE", "bWHALE"];
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for native_token in native_tokens.clone() {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        let pair_address = Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        );
        pair_tokens.push(pair_address);
    }

    // Provide liquidity into pools
    for (i, native_token) in native_tokens.clone().iter().enumerate() {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(500_000u128),
                },
                Coin {
                    denom: native_token.clone().to_string(),
                    amount: Uint128::new(500_000u128),
                },
            ],
        )
        .unwrap();
    }

    // add epochs to the fee distributor.

    // whale -> native
    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(200_000_000u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(200_000_000u128),
        }],
    )
    .unwrap();

    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678802400_000000000u64),
        chain_id: "".to_string(),
    });

    // Create new epoch, which triggers fee collection, aggregation and distribution
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // advance some time, but not enough to create a new epoch
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678802500_000000000u64), //less than a day
        chain_id: "".to_string(),
    });

    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(200_000_000u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(200_000_000u128),
        }],
    )
    .unwrap();

    let err = app
        .execute_contract(
            creator.sender.clone(),
            fee_distributor_address.clone(),
            &NewEpoch {},
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<fee_distributor::ContractError>().unwrap(),
        fee_distributor::ContractError::CurrentEpochNotExpired {}
    );
}

#[test]
fn aggregate_fees_unsuccessfully() {
    let creator = mock_creator();

    let mut app = mock_app();

    let fee_collector_id = store_fee_collector_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    // try to aggregate fees from an unauthorized address
    let err = app
        .execute_contract(
            Addr::unchecked("unauthorized"),
            fee_collector_address.clone(),
            &AggregateFees {
                asset_info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                aggregate_fees_for: FeesFor::Contracts { contracts: vec![] },
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );
}

#[test]
fn forward_fees_unsuccessfully() {
    let creator = mock_creator();

    let mut app = mock_app();

    let fee_collector_id = store_fee_collector_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    // try to forward fees from an unauthorized address
    let err = app
        .execute_contract(
            Addr::unchecked("unauthorized"),
            fee_collector_address.clone(),
            &ForwardFees {
                epoch: Default::default(),
                forward_fees_as: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<ContractError>().unwrap(),
        ContractError::Unauthorized {}
    );
}

#[test]
fn decrease_grace_period_fee_distributor() {
    let creator = mock_creator();
    let balances = vec![(
        creator.clone().sender,
        vec![
            coin(1_000_000_000, "usdc"),
            coin(1_000_000_000, "uwhale"),
            coin(1_000_000_000, "ampWHALE"),
            coin(1_000_000_000, "bWHALE"),
        ],
    )];

    let mut app = mock_app_with_balance(balances);

    let fee_collector_id = store_fee_collector_code(&mut app);
    let fee_distributor_id = store_fee_distributor_code(&mut app);
    let whale_lair_id = store_whale_lair_code(&mut app);
    let pool_factory_id = store_pool_factory_code(&mut app);
    let pool_router_id = store_pool_router_code(&mut app);
    let pair_id = store_pair_code(&mut app);
    let trio_id = store_trio_code(&mut app);
    let token_id = store_token_code(&mut app);
    let vault_factory_id = store_vault_factory_code(&mut app);
    let vault_id = store_vault_code(&mut app);

    let fee_collector_address = app
        .instantiate_contract(
            fee_collector_id,
            creator.clone().sender,
            &InstantiateMsg {},
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_factory_address = app
        .instantiate_contract(
            pool_factory_id,
            creator.clone().sender,
            &pool_network::factory::InstantiateMsg {
                pair_code_id: pair_id,
                trio_code_id: trio_id,
                token_code_id: token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "fee_collector",
            None,
        )
        .unwrap();

    let pool_router_address = app
        .instantiate_contract(
            pool_router_id,
            creator.clone().sender,
            &pool_network::router::InstantiateMsg {
                terraswap_factory: pool_factory_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let vault_factory_address = app
        .instantiate_contract(
            vault_factory_id,
            creator.clone().sender,
            &vault_network::vault_factory::InstantiateMsg {
                owner: creator.clone().sender.to_string(),
                vault_id,
                token_id,
                fee_collector_addr: fee_collector_address.to_string(),
            },
            &[],
            "pool_router",
            None,
        )
        .unwrap();

    let whale_lair_address = app
        .instantiate_contract(
            whale_lair_id,
            creator.clone().sender,
            &white_whale::whale_lair::InstantiateMsg {
                unbonding_period: Uint64::new(1_000_000_000_000u64),
                growth_rate: Decimal::one(),
                bonding_assets: vec![
                    AssetInfo::NativeToken {
                        denom: "ampWHALE".to_string(),
                    },
                    AssetInfo::NativeToken {
                        denom: "bWHALE".to_string(),
                    },
                ],
            },
            &[],
            "whale_lair",
            None,
        )
        .unwrap();

    let fee_distributor_address = app
        .instantiate_contract(
            fee_distributor_id,
            creator.clone().sender,
            &white_whale::fee_distributor::InstantiateMsg {
                bonding_contract_addr: whale_lair_address.clone().to_string(),
                fee_collector_addr: fee_collector_address.clone().to_string(),
                grace_period: Uint64::new(2),
                epoch_config: EpochConfig {
                    duration: Uint64::new(86_400_000_000_000u64), // a day
                    genesis_epoch: Uint64::new(1678802400_000000000u64), // March 14, 2023 2:00:00 PM
                },
                distribution_asset: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
            },
            &[],
            "fee_distributor",
            None,
        )
        .unwrap();

    // add pool router address to the fee collector to be able to aggregate fees
    app.execute_contract(
        creator.sender.clone(),
        fee_collector_address.clone(),
        &UpdateConfig {
            owner: None,
            pool_router: Some(pool_router_address.to_string()),
            fee_distributor: Some(fee_distributor_address.to_string()),
            pool_factory: Some(pool_factory_address.to_string()),
            vault_factory: Some(vault_factory_address.to_string()),
        },
        &[],
    )
    .unwrap();

    // add native tokens to the factory
    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "uwhale".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "uwhale".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "usdc".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "ampWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "ampWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    app.execute_contract(
        creator.sender.clone(),
        pool_factory_address.clone(),
        &AddNativeTokenDecimals {
            denom: "bWHALE".to_string(),
            decimals: 6,
        },
        &[Coin {
            denom: "bWHALE".to_string(),
            amount: Uint128::new(1u128),
        }],
    )
    .unwrap();

    // Create few pools
    let native_tokens: Vec<&str> = vec!["usdc", "ampWHALE", "bWHALE"];
    let mut pair_tokens: Vec<Addr> = Vec::new();
    for native_token in native_tokens.clone() {
        let res = app
            .execute_contract(
                creator.sender.clone(),
                pool_factory_address.clone(),
                &CreatePair {
                    asset_infos: [
                        AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                    ],
                    pool_fees: PoolFee {
                        protocol_fee: Fee {
                            share: Decimal::percent(5u64),
                        },
                        swap_fee: Fee {
                            share: Decimal::percent(7u64),
                        },
                        burn_fee: Fee {
                            share: Decimal::zero(),
                        },
                    },
                    pair_type: PairType::ConstantProduct,
                    token_factory_lp: false,
                },
                &[],
            )
            .unwrap();

        let pair_address = Addr::unchecked(
            res.events
                .last()
                .unwrap()
                .attributes
                .clone()
                .get(1)
                .unwrap()
                .clone()
                .value,
        );
        pair_tokens.push(pair_address);
    }

    // Provide liquidity into pools
    for (i, native_token) in native_tokens.clone().iter().enumerate() {
        app.execute_contract(
            creator.sender.clone(),
            pair_tokens[i].clone(),
            &pool_network::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                    Asset {
                        info: AssetInfo::NativeToken {
                            denom: native_token.clone().to_string(),
                        },
                        amount: Uint128::new(500_000u128),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
            },
            &[
                Coin {
                    denom: "uwhale".to_string(),
                    amount: Uint128::new(500_000u128),
                },
                Coin {
                    denom: native_token.clone().to_string(),
                    amount: Uint128::new(500_000u128),
                },
            ],
        )
        .unwrap();
    }

    // add epochs to the fee distributor.

    // whale -> native
    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(200_000_000u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(200_000_000u128),
        }],
    )
    .unwrap();

    // advance the time to one day after the first epoch was created
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678888800_000000000u64),
        chain_id: "".to_string(),
    });

    // Create new epoch, which triggers fee collection, aggregation and distribution
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // When creating the second epoch, the first one will be expiring since the grace_period was set to 1/.
    // Make sure the available tokens on the expiring epoch are transferred to the second one.
    app.execute_contract(
        creator.sender.clone(),
        pair_tokens[0].clone(),
        &pool_network::pair::ExecuteMsg::Swap {
            offer_asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(200_000_000u128),
            },
            belief_price: None,
            max_spread: None,
            to: None,
        },
        &[Coin {
            denom: "usdc".to_string(),
            amount: Uint128::new(200_000_000u128),
        }],
    )
    .unwrap();

    // advance the time to one day after the first epoch was created
    app.set_block(BlockInfo {
        height: 123456789u64,
        time: Timestamp::from_nanos(1678888800_000000000u64),
        chain_id: "".to_string(),
    });

    // Create new epoch, which triggers fee collection, aggregation and distribution
    app.execute_contract(
        creator.sender.clone(),
        fee_distributor_address.clone(),
        &NewEpoch {},
        &[],
    )
    .unwrap();

    // try updating the grace_period on the config to 1, cannot be decreased
    let err = app
        .execute_contract(
            creator.sender.clone(),
            fee_distributor_address.clone(),
            &white_whale::fee_distributor::ExecuteMsg::UpdateConfig {
                owner: None,
                bonding_contract_addr: None,
                fee_collector_addr: None,
                grace_period: Some(Uint64::one()),
                distribution_asset: None,
                epoch_config: None,
            },
            &[],
        )
        .unwrap_err();

    assert_eq!(
        err.downcast::<fee_distributor::ContractError>().unwrap(),
        fee_distributor::ContractError::GracePeriodDecrease {}
    );
}

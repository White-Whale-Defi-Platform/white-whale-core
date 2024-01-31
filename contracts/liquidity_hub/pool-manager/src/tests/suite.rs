use std::collections::HashMap;

use white_whale_std::pool_manager::{Cw20HookMsg, SwapOperation};
use white_whale_std::pool_manager::{ExecuteMsg, InstantiateMsg, NPairInfo, QueryMsg};

use anyhow::{Ok, Result as AnyResult};
use cosmwasm_std::{
    coin, to_json_binary, Addr, Coin, Decimal, Deps, Empty, StdResult, Timestamp, Uint128, Uint64,
};
use cw20::{BalanceResponse, Cw20Coin, MinterResponse};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, Contract, ContractWrapper, Executor, Router,
    WasmKeeper,
};
use white_whale_std::pool_network::pair::{ReverseSimulationResponse, SimulationResponse};
use white_whale_std::{
    pool_network::{
        asset::{Asset, AssetInfo, PairType},
        pair::PoolFee,
    },
    vault_manager::LpTokenType,
};

use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
fn contract_pool_manager() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );

    Box::new(contract)
}

fn cw20_token_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );

    Box::new(contract)
}

/// Creates the whale lair contract
pub fn whale_lair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        whale_lair::contract::execute,
        whale_lair::contract::instantiate,
        whale_lair::contract::query,
    )
    .with_migrate(whale_lair::contract::migrate);

    Box::new(contract)
}

pub struct TestingSuite {
    app: App<BankKeeper, MockApiBech32>,
    pub senders: [Addr; 3],
    pub whale_lair_addr: Addr,
    pub vault_manager_addr: Addr,
    pub cw20_tokens: Vec<Addr>,
}

/// TestingSuite helpers
impl TestingSuite {
    pub(crate) fn creator(&mut self) -> Addr {
        self.senders.first().unwrap().clone()
    }

    pub(crate) fn set_time(&mut self, timestamp: Timestamp) -> &mut Self {
        let mut block_info = self.app.block_info();
        block_info.time = timestamp;
        self.app.set_block(block_info);

        self
    }

    pub(crate) fn get_time(&mut self) -> Timestamp {
        self.app.block_info().time
    }

    pub(crate) fn increase_allowance(
        &mut self,
        sender: Addr,
        cw20contract: Addr,
        allowance: Uint128,
        spender: Addr,
    ) -> &mut Self {
        let msg = cw20_base::msg::ExecuteMsg::IncreaseAllowance {
            spender: spender.to_string(),
            amount: allowance,
            expires: None,
        };

        self.app
            .execute_contract(sender, cw20contract, &msg, &vec![])
            .unwrap();

        self
    }
}

/// Instantiate
impl TestingSuite {
    pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
        let sender_1 = Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3");
        let sender_2 = Addr::unchecked("migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40");
        let sender_3 = Addr::unchecked("migaloo1ludaslnu24p5eftw499f7ngsc2jkzqdsrvxt75");

        let bank = BankKeeper::new();

        let balances = vec![
            (sender_1.clone(), initial_balance.clone()),
            (sender_2.clone(), initial_balance.clone()),
            (sender_3.clone(), initial_balance.clone()),
        ];

        let app = AppBuilder::new()
            .with_api(MockApiBech32::new("migaloo"))
            .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
            .with_bank(bank)
            .build(|router, _api, storage| {
                balances.into_iter().for_each(|(account, amount)| {
                    router.bank.init_balance(storage, &account, amount).unwrap()
                });
            });

        Self {
            app,
            senders: [sender_1, sender_2, sender_3],
            whale_lair_addr: Addr::unchecked(""),
            vault_manager_addr: Addr::unchecked(""),
            cw20_tokens: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate(
        &mut self,
        whale_lair_addr: String,
        _lp_token_type: LpTokenType,
        _vault_creation_fee: Asset,
    ) -> &mut Self {
        let cw20_token_id = self.app.store_code(cw20_token_contract());
        let msg = InstantiateMsg {
            fee_collector_addr: whale_lair_addr,
            token_code_id: cw20_token_id,
            pair_code_id: cw20_token_id,
            owner: self.creator().to_string(),
            pool_creation_fee: Asset {
                amount: Uint128::from(1_000u128),
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            },
        };

        let vault_manager_id = self.app.store_code(contract_pool_manager());

        let creator = self.creator().clone();

        self.vault_manager_addr = self
            .app
            .instantiate_contract(
                vault_manager_id,
                creator.clone(),
                &msg,
                &[],
                "mock pool manager",
                Some(creator.into_string()),
            )
            .unwrap();
        self
    }

    #[track_caller]
    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        self.create_whale_lair();
        self.create_cw20_token();

        // 17 May 2023 17:00:00 UTC
        let timestamp = Timestamp::from_seconds(1684342800u64);
        self.set_time(timestamp);

        self.instantiate(
            self.whale_lair_addr.to_string(),
            LpTokenType::TokenFactory,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
        )
    }

    #[track_caller]
    pub(crate) fn instantiate_with_cw20_lp_token(&mut self) -> &mut Self {
        self.create_whale_lair();
        let cw20_code_id = self.create_cw20_token();
        println!("cw20_code_id: {}", self.whale_lair_addr);
        // 17 May 2023 17:00:00 UTC
        let timestamp = Timestamp::from_seconds(1684342800u64);
        self.set_time(timestamp);

        self.instantiate(
            self.whale_lair_addr.to_string(),
            LpTokenType::Cw20(cw20_code_id),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
        )
    }

    fn create_whale_lair(&mut self) {
        let whale_lair_id = self.app.store_code(whale_lair_contract());

        // create whale lair
        let msg = white_whale_std::whale_lair::InstantiateMsg {
            unbonding_period: Uint64::new(86400u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
            ],
        };

        let creator = self.creator().clone();

        self.whale_lair_addr = self
            .app
            .instantiate_contract(
                whale_lair_id,
                creator.clone(),
                &msg,
                &[],
                "White Whale Lair".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }

    #[track_caller]
    pub fn create_cw20_token(&mut self) -> u64 {
        let msg = white_whale_std::pool_network::token::InstantiateMsg {
            name: "mocktoken".to_string(),
            symbol: "MOCK".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: self.senders[0].to_string(),
                    amount: Uint128::new(1_000_000_000_000u128),
                },
                Cw20Coin {
                    address: self.senders[1].to_string(),
                    amount: Uint128::new(1_000_000_000_000u128),
                },
                Cw20Coin {
                    address: self.senders[2].to_string(),
                    amount: Uint128::new(1_000_000_000_000u128),
                },
            ],
            mint: Some(MinterResponse {
                minter: self.senders[0].to_string(),
                cap: None,
            }),
        };

        let cw20_token_id = self.app.store_code(cw20_token_contract());

        let creator = self.creator().clone();

        self.cw20_tokens.append(&mut vec![self
            .app
            .instantiate_contract(
                cw20_token_id,
                creator.clone(),
                &msg,
                &[],
                "mock cw20 token",
                Some(creator.into_string()),
            )
            .unwrap()]);
        cw20_token_id
    }

    #[track_caller]
    pub fn add_native_token_decimals(
        &mut self,
        sender: Addr,
        native_token_denom: String,
        decimals: u8,
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::AddNativeTokenDecimals {
            denom: native_token_denom.clone(),
            decimals: decimals,
        };

        let _creator = self.creator().clone();

        self.app
            .execute_contract(
                sender,
                self.vault_manager_addr.clone(),
                &msg,
                &[Coin {
                    denom: native_token_denom.to_string(),
                    amount: Uint128::from(1u128),
                }],
            )
            .unwrap();

        self
    }
}

/// execute messages
impl TestingSuite {
    #[track_caller]
    pub(crate) fn update_ownership(
        &mut self,
        sender: Addr,
        action: cw_ownable::Action,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::UpdateOwnership(action);

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn provide_liquidity(
        &mut self,
        sender: Addr,
        pair_identifier: String,
        assets: Vec<Asset>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::ProvideLiquidity {
            assets,
            pair_identifier,
            slippage_tolerance: None,
            receiver: None,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn swap(
        &mut self,
        sender: Addr,
        offer_asset: Asset,
        ask_asset: AssetInfo,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        pair_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::Swap {
            offer_asset,
            ask_asset,
            belief_price,
            max_spread,
            to,
            pair_identifier,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    // #[track_caller]
    // pub(crate) fn execute_swap_operations(
    //     &mut self,
    //     sender: Addr,
    //     operations: Vec<SwapOperation>,
    //     minimum_receive: Option<Uint128>,
    //     to: Option<String>,
    //     max_spread: Option<Decimal>,
    //     funds: Vec<Coin>,
    //     result: impl Fn(Result<AppResponse, anyhow::Error>),
    // ) -> &mut Self {
    //     let msg = white_whale_std::pool_manager::ExecuteMsg::ExecuteSwapOperations { operations, minimum_receive, to, max_spread };

    //     result(
    //         self.app
    //             .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
    //     );

    //     self
    // }

    #[track_caller]
    pub(crate) fn create_pair(
        &mut self,
        sender: Addr,
        asset_infos: Vec<AssetInfo>,
        pool_fees: PoolFee,
        pair_type: PairType,
        token_factory_lp: bool,
        pair_identifier: Option<String>,
        pair_creation_fee_funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::CreatePair {
            asset_infos,
            pool_fees,
            pair_type,
            token_factory_lp,
            pair_identifier,
        };

        result(self.app.execute_contract(
            sender,
            self.vault_manager_addr.clone(),
            &msg,
            &pair_creation_fee_funds,
        ));

        self
    }

    #[track_caller]
    pub(crate) fn withdraw_liquidity(
        &mut self,
        sender: Addr,
        pair_identifier: String,
        assets: Vec<Asset>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::WithdrawLiquidity {
            assets,
            pair_identifier,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn withdraw_liquidity_cw20(
        &mut self,
        sender: Addr,
        pair_identifier: String,
        assets: Vec<Asset>,
        amount: Uint128,
        liquidity_token: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        // Prepare a CW20 Transfer message with a CW20HookMsg to withdraw liquidity

        // Send the cw20 amount with a message
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: self.vault_manager_addr.to_string(),
            amount: amount,
            msg: to_json_binary(&Cw20HookMsg::WithdrawLiquidity {
                pair_identifier: pair_identifier,
            })
            .unwrap(),
        };

        result(
            self.app
                .execute_contract(sender, liquidity_token, &msg, &[]),
        );

        self
    }
}

/// queries
impl TestingSuite {
    pub(crate) fn query_ownership(
        &mut self,
        result: impl Fn(StdResult<cw_ownable::Ownership<String>>),
    ) -> &mut Self {
        let ownership_response: StdResult<cw_ownable::Ownership<String>> =
            self.app.wrap().query_wasm_smart(
                &self.vault_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Ownership {},
            );

        result(ownership_response);

        self
    }

    pub(crate) fn query_balance(
        &mut self,
        addr: String,
        denom: String,
        result: impl Fn(StdResult<Coin>),
    ) -> &mut Self {
        let balance_resp: StdResult<Coin> = self.app.wrap().query_balance(&addr, denom);

        result(balance_resp);

        self
    }

    pub(crate) fn query_pair_info(
        &mut self,
        pair_identifier: String,
        result: impl Fn(StdResult<NPairInfo>),
    ) -> &mut Self {
        let pair_info_response: StdResult<NPairInfo> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::Pair {
                pair_identifier: pair_identifier,
            },
        );

        result(pair_info_response);

        self
    }

    pub(crate) fn query_simulation(
        &mut self,
        pair_identifier: String,
        offer_asset: Asset,
        ask_asset: AssetInfo,
        result: impl Fn(StdResult<SimulationResponse>),
    ) -> &mut Self {
        let pair_info_response: StdResult<SimulationResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::Simulation {
                offer_asset,
                ask_asset: Asset {
                    amount: Uint128::zero(),
                    info: ask_asset,
                },
                pair_identifier,
            },
        );

        result(pair_info_response);

        self
    }

    pub(crate) fn query_reverse_simulation(
        &mut self,
        pair_identifier: String,
        offer_asset: AssetInfo,
        ask_asset: Asset,
        result: impl Fn(StdResult<ReverseSimulationResponse>),
    ) -> &mut Self {
        let pair_info_response: StdResult<ReverseSimulationResponse> =
            self.app.wrap().query_wasm_smart(
                &self.vault_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::ReverseSimulation {
                    offer_asset: Asset {
                        amount: Uint128::zero(),
                        info: offer_asset,
                    },
                    ask_asset,
                    pair_identifier,
                },
            );

        result(pair_info_response);

        self
    }

    pub(crate) fn query_amount_of_lp_token(
        &mut self,
        identifier: String,
        sender: String,
        result: impl Fn(StdResult<Uint128>),
    ) -> &mut Self {
        // Get the LP token from Config
        let lp_token_response: NPairInfo = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.vault_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Pair {
                    pair_identifier: identifier,
                },
            )
            .unwrap();

        // Get balance of LP token, if native we can just query balance otherwise we need to go to cw20

        let balance = match lp_token_response.liquidity_token {
            AssetInfo::NativeToken { denom } => {
                let balance_response: Uint128 =
                    self.app.wrap().query_balance(sender, denom).unwrap().amount;

                balance_response
            }
            AssetInfo::Token { contract_addr } => {
                let balance_response: BalanceResponse = self
                    .app
                    .wrap()
                    .query_wasm_smart(
                        &contract_addr,
                        &cw20_base::msg::QueryMsg::Balance { address: sender },
                    )
                    .unwrap();

                balance_response.balance
            }
        };

        result(Result::Ok(balance));
        self
    }

    pub(crate) fn query_lp_token(&mut self, identifier: String, sender: String) -> AssetInfo {
        // Get the LP token from Config
        let lp_token_response: NPairInfo = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.vault_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Pair {
                    pair_identifier: identifier,
                },
            )
            .unwrap();

        // Get balance of LP token, if native we can just query balance otherwise we need to go to cw20
        lp_token_response.liquidity_token
    }
}

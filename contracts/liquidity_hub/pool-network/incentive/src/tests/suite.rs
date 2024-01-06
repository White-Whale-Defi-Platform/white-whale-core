use cosmwasm_std::{Addr, Coin, StdResult, Timestamp, Uint128};
use cw20::{BalanceResponse, Cw20Coin, MinterResponse};
use cw_multi_test::{App, AppBuilder, AppResponse, BankKeeper, Executor};

use white_whale::fee_distributor::EpochResponse;
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::incentive::{
    Curve, Flow, FlowIdentifier, FlowResponse, GlobalWeightResponse, PositionsResponse,
    RewardsResponse, RewardsShareResponse,
};
use white_whale::pool_network::incentive_factory::{
    IncentiveResponse, IncentivesResponse, InstantiateMsg,
};

use crate::tests::suite_contracts::{
    cw20_token_contract, fee_collector_contract, fee_distributor_mock_contract, incentive_contract,
    incentive_factory_contract,
};

pub struct TestingSuite {
    app: App,
    pub senders: [Addr; 3],
    pub incentive_factory_addr: Addr,
    pub fee_distributor_addr: Addr,
    pub cw20_tokens: Vec<Addr>,
}

/// helpers
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
            .execute_contract(sender, cw20contract, &msg, &[])
            .unwrap();

        self
    }
}

/// instantiate / execute messages
impl TestingSuite {
    pub(crate) fn default() -> Self {
        let sender_1 = Addr::unchecked("alice");
        let sender_2 = Addr::unchecked("bob");
        let sender_3 = Addr::unchecked("carol");

        Self {
            app: App::default(),
            senders: [sender_1, sender_2, sender_3],
            incentive_factory_addr: Addr::unchecked(""),
            fee_distributor_addr: Addr::unchecked(""),
            cw20_tokens: vec![],
        }
    }

    pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
        let sender_1 = Addr::unchecked("alice");
        let sender_2 = Addr::unchecked("bob");
        let sender_3 = Addr::unchecked("carol");

        let bank = BankKeeper::new();

        let balances = vec![
            (sender_1.clone(), initial_balance.clone()),
            (sender_2.clone(), initial_balance.clone()),
            (sender_3.clone(), initial_balance.clone()),
        ];

        let app = AppBuilder::new()
            .with_bank(bank)
            .build(|router, _api, storage| {
                balances.into_iter().for_each(|(account, amount)| {
                    router.bank.init_balance(storage, &account, amount).unwrap()
                });
            });

        Self {
            app,
            senders: [sender_1, sender_2, sender_3],
            incentive_factory_addr: Addr::unchecked(""),
            fee_distributor_addr: Addr::unchecked(""),
            cw20_tokens: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate_default_native_fee(&mut self) -> &mut Self {
        let incentive_id = self.app.store_code(incentive_contract());
        let fee_collector_addr =
            instantiate_contract(self, InstatiateContract::FeeCollector {}).unwrap();
        let fee_distributor_addr =
            instantiate_contract(self, InstatiateContract::FeeDistributor {}).unwrap();

        let cw20_token = instantiate_contract(
            self,
            InstatiateContract::CW20 {
                name: "uLP".to_string(),
                symbol: "uLP".to_string(),
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
            },
        )
        .unwrap();

        self.cw20_tokens = vec![cw20_token.clone()];

        // 17 May 2023 17:00:00 UTC
        let timestamp = Timestamp::from_seconds(1684342800u64);
        self.set_time(timestamp);

        self.instantiate(
            fee_collector_addr.to_string(),
            fee_distributor_addr.to_string(),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            7u64,
            incentive_id,
            14, // 2 weeks
            86400,
            259200,
        )
    }

    #[track_caller]
    pub(crate) fn instantiate_default_cw20_fee(&mut self) -> &mut Self {
        let incentive_id = self.app.store_code(incentive_contract());
        let fee_collector_addr =
            instantiate_contract(self, InstatiateContract::FeeCollector {}).unwrap();
        let fee_distributor_addr =
            instantiate_contract(self, InstatiateContract::FeeDistributor {}).unwrap();

        let cw20_token = instantiate_contract(
            self,
            InstatiateContract::CW20 {
                name: "uLP".to_string(),
                symbol: "uLP".to_string(),
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
            },
        )
        .unwrap();

        self.cw20_tokens = vec![cw20_token.clone()];

        // 17 May 2023 17:00:00 UTC
        let timestamp = Timestamp::from_seconds(1684342800u64);
        self.set_time(timestamp);

        self.instantiate(
            fee_collector_addr.to_string(),
            fee_distributor_addr.to_string(),
            Asset {
                info: AssetInfo::Token {
                    contract_addr: cw20_token.to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
            7u64,
            incentive_id,
            100u64,
            86400,
            259200,
        )
    }

    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn instantiate(
        &mut self,
        fee_collector_addr: String,
        fee_distributor_addr: String,
        create_flow_fee: Asset,
        max_concurrent_flows: u64,
        incentive_code_id: u64,
        max_flow_epoch_buffer: u64,
        min_unbonding_duration: u64,
        max_unbonding_duration: u64,
    ) -> &mut Self {
        let incentive_factory_addr = instantiate_contract(
            self,
            InstatiateContract::IncentiveFactory {
                fee_collector_addr,
                fee_distributor_addr: fee_distributor_addr.clone(),
                create_flow_fee,
                max_concurrent_flows,
                incentive_code_id,
                max_flow_epoch_buffer,
                min_unbonding_duration,
                max_unbonding_duration,
            },
        )
        .unwrap();

        self.incentive_factory_addr = incentive_factory_addr;
        self.fee_distributor_addr = Addr::unchecked(fee_distributor_addr);
        self
    }

    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn instantiate_err(
        &mut self,
        fee_collector_addr: String,
        fee_distributor_addr: String,
        create_flow_fee: Asset,
        max_concurrent_flows: u64,
        incentive_code_id: u64,
        max_flow_epoch_buffer: u64,
        min_unbonding_duration: u64,
        max_unbonding_duration: u64,
        error: impl Fn(anyhow::Error),
    ) -> &mut Self {
        let err = instantiate_contract(
            self,
            InstatiateContract::IncentiveFactory {
                fee_collector_addr,
                fee_distributor_addr,
                create_flow_fee,
                max_concurrent_flows,
                incentive_code_id,
                max_flow_epoch_buffer,
                min_unbonding_duration,
                max_unbonding_duration,
            },
        )
        .unwrap_err();

        error(err);

        self
    }

    #[track_caller]
    pub(crate) fn create_lp_tokens(&mut self) -> &mut Self {
        let mut lp_tokens = self.cw20_tokens.clone();

        for _ in 0..9 {
            let cw20_token = instantiate_contract(
                self,
                InstatiateContract::CW20 {
                    name: "uLP".to_string(),
                    symbol: "uLP".to_string(),
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
                },
            )
            .unwrap();

            lp_tokens.push(cw20_token.clone());
        }

        self.cw20_tokens = lp_tokens;

        self
    }
}

/// execute messages
impl TestingSuite {
    pub(crate) fn create_incentive(
        &mut self,
        sender: Addr,
        lp_address: AssetInfo,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive_factory::ExecuteMsg::CreateIncentive {
            lp_asset: lp_address,
        };

        result(
            self.app
                .execute_contract(sender, self.incentive_factory_addr.clone(), &msg, &[]),
        );

        self
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn open_incentive_flow(
        &mut self,
        sender: Addr,
        incentive_addr: Addr,
        start_epoch: Option<u64>,
        end_epoch: Option<u64>,
        curve: Option<Curve>,
        flow_asset: Asset,
        flow_label: Option<String>,
        funds: &[Coin],
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::OpenFlow {
            start_epoch,
            end_epoch,
            curve,
            flow_asset,
            flow_label,
        };

        result(
            self.app
                .execute_contract(sender, incentive_addr, &msg, funds),
        );

        self
    }

    pub(crate) fn close_incentive_flow(
        &mut self,
        sender: Addr,
        incentive_addr: Addr,
        flow_identifier: FlowIdentifier,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::CloseFlow { flow_identifier };

        result(self.app.execute_contract(sender, incentive_addr, &msg, &[]));

        self
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn open_incentive_position(
        &mut self,
        sender: Addr,
        incentive_addr: Addr,
        amount: Uint128,
        unbonding_duration: u64,
        receiver: Option<String>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::OpenPosition {
            amount,
            unbonding_duration,
            receiver,
        };

        result(
            self.app
                .execute_contract(sender, incentive_addr, &msg, &funds),
        );

        self
    }

    pub(crate) fn close_incentive_position(
        &mut self,
        sender: Addr,
        incentive_addr: Addr,
        unbonding_duration: u64,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg =
            white_whale::pool_network::incentive::ExecuteMsg::ClosePosition { unbonding_duration };

        result(self.app.execute_contract(sender, incentive_addr, &msg, &[]));

        self
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn expand_incentive_position(
        &mut self,
        sender: Addr,
        incentive_addr: Addr,
        amount: Uint128,
        unbonding_duration: u64,
        receiver: Option<String>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::ExpandPosition {
            amount,
            unbonding_duration,
            receiver,
        };

        result(
            self.app
                .execute_contract(sender, incentive_addr, &msg, &funds),
        );

        self
    }

    pub(crate) fn claim(
        &mut self,
        incentive_addr: Addr,
        sender: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::Claim {};
        println!("-------------- claiming {}", sender);
        result(self.app.execute_contract(sender, incentive_addr, &msg, &[]));

        self
    }

    pub(crate) fn withdraw(
        &mut self,
        incentive_addr: Addr,
        sender: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::Withdraw {};
        result(self.app.execute_contract(sender, incentive_addr, &msg, &[]));

        self
    }

    pub(crate) fn create_epochs_on_fee_distributor(
        &mut self,
        epoch_amount: u64,
        incentive_addresses_to_snapshot_global_weight_for: Vec<Addr>,
    ) -> &mut Self {
        let msg = white_whale::fee_distributor::ExecuteMsg::NewEpoch {};

        for _ in 0..epoch_amount {
            self.app
                .execute_contract(
                    self.senders[0].clone(),
                    self.fee_distributor_addr.clone(),
                    &msg,
                    &[],
                )
                .unwrap();

            incentive_addresses_to_snapshot_global_weight_for
                .iter()
                .for_each(|incentive_addr| {
                    self.take_global_weight_snapshot(incentive_addr.clone(), |result| {
                        result.unwrap();
                    });
                });
        }

        self
    }

    pub(crate) fn create_epochs_on_fee_distributor_without_snapshot_on_incentive(
        &mut self,
        epoch_amount: u64,
    ) -> &mut Self {
        let msg = white_whale::fee_distributor::ExecuteMsg::NewEpoch {};

        for _ in 0..epoch_amount {
            self.app
                .execute_contract(
                    self.senders[0].clone(),
                    self.fee_distributor_addr.clone(),
                    &msg,
                    &[],
                )
                .unwrap();
        }

        self
    }

    pub(crate) fn take_global_weight_snapshot(
        &mut self,
        incentive_addr: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::TakeGlobalWeightSnapshot {};

        result(self.app.execute_contract(
            self.senders[0].clone(),
            incentive_addr.clone(),
            &msg,
            &[],
        ));

        self
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn expand_flow(
        &mut self,
        sender: Addr,
        incentive_addr: Addr,
        flow_identifier: FlowIdentifier,
        end_epoch: Option<u64>,
        flow_asset: Asset,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::pool_network::incentive::ExecuteMsg::ExpandFlow {
            flow_identifier,
            end_epoch,
            flow_asset,
        };

        result(
            self.app
                .execute_contract(sender, incentive_addr.clone(), &msg, &funds),
        );

        self
    }
}

/// queries
impl TestingSuite {
    pub(crate) fn query_current_epoch(
        &mut self,
        result: impl Fn(StdResult<EpochResponse>),
    ) -> &mut Self {
        let current_epoch_response: StdResult<EpochResponse> = self.app.wrap().query_wasm_smart(
            &self.fee_distributor_addr,
            &white_whale::fee_distributor::QueryMsg::CurrentEpoch {},
        );

        result(current_epoch_response);

        self
    }

    pub(crate) fn query_incentive(
        &mut self,
        lp_address: AssetInfo,
        result: impl Fn(StdResult<IncentiveResponse>),
    ) -> &mut Self {
        let incentive_response: StdResult<IncentiveResponse> = self.app.wrap().query_wasm_smart(
            &self.incentive_factory_addr,
            &white_whale::pool_network::incentive_factory::QueryMsg::Incentive {
                lp_asset: lp_address,
            },
        );

        result(incentive_response);

        self
    }

    pub(crate) fn query_incentives(
        &mut self,
        start_after: Option<AssetInfo>,
        limit: Option<u32>,
        result: impl Fn(StdResult<IncentivesResponse>),
    ) -> &mut Self {
        let incentive_response: StdResult<IncentivesResponse> = self.app.wrap().query_wasm_smart(
            &self.incentive_factory_addr,
            &white_whale::pool_network::incentive_factory::QueryMsg::Incentives {
                start_after,
                limit,
            },
        );

        result(incentive_response);

        self
    }

    pub(crate) fn query_incentive_global_weight(
        &mut self,
        incentive_addr: Addr,
        epoch_id: u64,
        result: impl Fn(StdResult<GlobalWeightResponse>),
    ) -> &mut Self {
        let global_weight_response: StdResult<GlobalWeightResponse> =
            self.app.wrap().query_wasm_smart(
                incentive_addr,
                &white_whale::pool_network::incentive::QueryMsg::GlobalWeight { epoch_id },
            );

        result(global_weight_response);

        self
    }

    pub(crate) fn query_current_epoch_rewards_share(
        &mut self,
        incentive_addr: Addr,
        address: Addr,
        result: impl Fn(StdResult<RewardsShareResponse>),
    ) -> &mut Self {
        let current_epoch_rewards_share: StdResult<RewardsShareResponse> =
            self.app.wrap().query_wasm_smart(
                incentive_addr,
                &white_whale::pool_network::incentive::QueryMsg::CurrentEpochRewardsShare {
                    address: address.to_string(),
                },
            );

        result(current_epoch_rewards_share);

        self
    }

    pub(crate) fn query_flow(
        &mut self,
        incentive_addr: Addr,
        flow_identifier: FlowIdentifier,
        result: impl Fn(StdResult<Option<FlowResponse>>),
    ) -> &mut Self {
        let flow_response: StdResult<Option<FlowResponse>> = self.app.wrap().query_wasm_smart(
            incentive_addr,
            &white_whale::pool_network::incentive::QueryMsg::Flow {
                flow_identifier,
                start_epoch: None,
                end_epoch: None,
            },
        );

        result(flow_response);

        self
    }

    pub(crate) fn query_flows(
        &mut self,
        incentive_addr: Addr,
        start_epoch: Option<u64>,
        end_epoch: Option<u64>,
        result: impl Fn(StdResult<Vec<Flow>>),
    ) -> &mut Self {
        let flows_response: StdResult<Vec<Flow>> = self.app.wrap().query_wasm_smart(
            incentive_addr,
            &white_whale::pool_network::incentive::QueryMsg::Flows {
                start_epoch,
                end_epoch,
            },
        );

        result(flows_response);

        self
    }

    pub(crate) fn query_positions(
        &mut self,
        incentive_addr: Addr,
        address: Addr,
        result: impl Fn(StdResult<PositionsResponse>),
    ) -> &mut Self {
        let positions_response: StdResult<PositionsResponse> = self.app.wrap().query_wasm_smart(
            incentive_addr,
            &white_whale::pool_network::incentive::QueryMsg::Positions {
                address: address.to_string(),
            },
        );

        result(positions_response);

        self
    }

    pub(crate) fn query_rewards(
        &mut self,
        incentive_addr: Addr,
        address: Addr,
        result: impl Fn(StdResult<RewardsResponse>),
    ) -> &mut Self {
        let rewards_response: StdResult<RewardsResponse> = self.app.wrap().query_wasm_smart(
            incentive_addr,
            &white_whale::pool_network::incentive::QueryMsg::Rewards {
                address: address.to_string(),
            },
        );

        result(rewards_response);

        self
    }

    pub(crate) fn query_incentive_factory_config(
        &mut self,
        result: impl Fn(StdResult<white_whale::pool_network::incentive_factory::ConfigResponse>),
    ) -> &mut Self {
        let config_response: StdResult<
            white_whale::pool_network::incentive_factory::ConfigResponse,
        > = self.app.wrap().query_wasm_smart(
            self.incentive_factory_addr.clone(),
            &white_whale::pool_network::incentive_factory::QueryMsg::Config {},
        );

        result(config_response);
        self
    }

    pub(crate) fn query_incentive_config(
        &mut self,
        incentive: Addr,
        result: impl Fn(StdResult<white_whale::pool_network::incentive::ConfigResponse>),
    ) -> &mut Self {
        let config_response: StdResult<white_whale::pool_network::incentive::ConfigResponse> =
            self.app.wrap().query_wasm_smart(
                incentive,
                &white_whale::pool_network::incentive::QueryMsg::Config {},
            );

        result(config_response);
        self
    }

    pub(crate) fn query_funds(
        &mut self,
        address: Addr,
        asset: AssetInfo,
        result: impl Fn(Uint128),
    ) -> &mut Self {
        let funds = match asset {
            AssetInfo::Token { contract_addr } => {
                let balance_response: StdResult<BalanceResponse> =
                    self.app.wrap().query_wasm_smart(
                        contract_addr,
                        &cw20_base::msg::QueryMsg::Balance {
                            address: address.to_string(),
                        },
                    );

                balance_response.unwrap().balance
            }
            AssetInfo::NativeToken { denom } => {
                let coin: StdResult<Coin> = self.app.wrap().query_balance(address, denom);
                coin.unwrap().amount
            }
        };

        result(funds);
        self
    }
}

enum InstatiateContract {
    IncentiveFactory {
        fee_collector_addr: String,
        fee_distributor_addr: String,
        create_flow_fee: Asset,
        max_concurrent_flows: u64,
        incentive_code_id: u64,
        max_flow_epoch_buffer: u64,
        min_unbonding_duration: u64,
        max_unbonding_duration: u64,
    },
    FeeCollector,
    FeeDistributor,
    CW20 {
        name: String,
        symbol: String,
        decimals: u8,
        initial_balances: Vec<Cw20Coin>,
        mint: Option<MinterResponse>,
    },
}

fn instantiate_contract(
    suite: &mut TestingSuite,
    instantiate_contract: InstatiateContract,
) -> anyhow::Result<Addr> {
    match instantiate_contract {
        InstatiateContract::IncentiveFactory {
            fee_collector_addr,
            fee_distributor_addr,
            create_flow_fee,
            max_concurrent_flows,
            incentive_code_id,
            max_flow_epoch_buffer,
            min_unbonding_duration,
            max_unbonding_duration,
        } => {
            let msg = InstantiateMsg {
                fee_collector_addr,
                fee_distributor_addr,
                create_flow_fee,
                max_concurrent_flows,
                incentive_code_id,
                max_flow_epoch_buffer,
                min_unbonding_duration,
                max_unbonding_duration,
            };

            let incentive_factory_id = suite.app.store_code(incentive_factory_contract());

            suite.app.instantiate_contract(
                incentive_factory_id,
                suite.senders[0].clone(),
                &msg,
                &[],
                "mock incentive factory",
                Some(suite.senders[0].clone().into_string()),
            )
        }
        InstatiateContract::FeeCollector => {
            let msg = white_whale::fee_collector::InstantiateMsg {};

            let fee_collector_id = suite.app.store_code(fee_collector_contract());

            suite.app.instantiate_contract(
                fee_collector_id,
                suite.senders[0].clone(),
                &msg,
                &[],
                "mock fee collector",
                Some(suite.senders[0].clone().into_string()),
            )
        }
        InstatiateContract::CW20 {
            name,
            symbol,
            decimals,
            initial_balances,
            mint,
        } => {
            let msg = white_whale::pool_network::token::InstantiateMsg {
                name,
                symbol,
                decimals,
                initial_balances,
                mint,
            };

            let cw20_token_id = suite.app.store_code(cw20_token_contract());

            suite.app.instantiate_contract(
                cw20_token_id,
                suite.senders[0].clone(),
                &msg,
                &[],
                "mock cw20 token",
                Some(suite.senders[0].clone().into_string()),
            )
        }
        InstatiateContract::FeeDistributor => {
            let msg = fee_distributor_mock::msg::InstantiateMsg {};

            let fee_distributor_mock_id = suite.app.store_code(fee_distributor_mock_contract());

            suite.app.instantiate_contract(
                fee_distributor_mock_id,
                suite.senders[0].clone(),
                &msg,
                &[],
                "mock fee distributor",
                Some(suite.senders[0].clone().into_string()),
            )
        }
    }
}

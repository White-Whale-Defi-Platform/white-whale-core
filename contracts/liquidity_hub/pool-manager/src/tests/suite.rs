use cosmwasm_std::testing::MockStorage;
use white_whale_std::pool_manager::{Config, FeatureToggle, SwapOperation};
use white_whale_std::pool_manager::{InstantiateMsg, PairInfo};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, StdResult, Timestamp, Uint128, Uint64};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, Contract, ContractWrapper, DistributionKeeper,
    Executor, FailingModule, GovFailingModule, IbcFailingModule, StakeKeeper, WasmKeeper,
};
use white_whale_std::fee::PoolFee;
use white_whale_std::pool_network::asset::{AssetInfo, PairType};
use white_whale_std::pool_network::pair::{ReverseSimulationResponse, SimulationResponse};
use white_whale_testing::multi_test::stargate_mock::StargateMock;

use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use white_whale_std::lp_common::LP_SYMBOL;

fn contract_pool_manager() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
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

type OsmosisTokenFactoryApp = App<
    BankKeeper,
    MockApiBech32,
    MockStorage,
    FailingModule<Empty, Empty, Empty>,
    WasmKeeper<Empty, Empty>,
    StakeKeeper,
    DistributionKeeper,
    IbcFailingModule,
    GovFailingModule,
    StargateMock,
>;

pub struct TestingSuite {
    app: OsmosisTokenFactoryApp,
    pub senders: [Addr; 3],
    pub whale_lair_addr: Addr,
    pub pool_manager_addr: Addr,
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

    pub(crate) fn get_lp_denom(&self, pair_id: String) -> String {
        // TODO: this should have
        format!(
            "factory/{}/u{}.pool.{}.{}",
            self.pool_manager_addr, pair_id, pair_id, LP_SYMBOL
        )
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
            .with_stargate(StargateMock {})
            .build(|router, _api, storage| {
                balances.into_iter().for_each(|(account, amount)| {
                    router.bank.init_balance(storage, &account, amount).unwrap()
                });
            });

        Self {
            app,
            senders: [sender_1, sender_2, sender_3],
            whale_lair_addr: Addr::unchecked(""),
            pool_manager_addr: Addr::unchecked(""),
            cw20_tokens: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate(&mut self, whale_lair_addr: String) -> &mut Self {
        let msg = InstantiateMsg {
            fee_collector_addr: whale_lair_addr,
            pool_creation_fee: coin(1_000, "uusd"),
        };

        let pool_manager_id = self.app.store_code(contract_pool_manager());

        let creator = self.creator().clone();

        self.pool_manager_addr = self
            .app
            .instantiate_contract(
                pool_manager_id,
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

        // 17 May 2023 17:00:00 UTC
        let timestamp = Timestamp::from_seconds(1684342800u64);
        self.set_time(timestamp);

        self.instantiate(self.whale_lair_addr.to_string())
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
                .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn provide_liquidity(
        &mut self,
        sender: Addr,
        pair_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::ProvideLiquidity {
            pair_identifier,
            slippage_tolerance: None,
            receiver: None,
        };

        result(
            self.app
                .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn swap(
        &mut self,
        sender: Addr,
        offer_asset: Coin,
        ask_asset_denom: String,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        pair_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::Swap {
            offer_asset,
            ask_asset_denom,
            belief_price,
            max_spread,
            to,
            pair_identifier,
        };

        result(
            self.app
                .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute_swap_operations(
        &mut self,
        sender: Addr,
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        max_spread: Option<Decimal>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            to,
            max_spread,
        };

        result(
            self.app
                .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_pair(
        &mut self,
        sender: Addr,
        asset_denoms: Vec<String>,
        asset_decimals: Vec<u8>,
        pool_fees: PoolFee,
        pair_type: PairType,
        pair_identifier: Option<String>,
        pair_creation_fee_funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::CreatePair {
            asset_denoms,
            asset_decimals,
            pool_fees,
            pair_type,
            pair_identifier,
        };

        result(self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
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
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::WithdrawLiquidity { pair_identifier };

        result(
            self.app
                .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    /// Updates the configuration of the contract.
    ///
    /// Any parameters which are set to `None` when passed will not update
    /// the current configuration.
    #[track_caller]
    pub(crate) fn update_config(
        &mut self,
        sender: Addr,
        new_whale_lair_addr: Option<Addr>,
        new_pool_creation_fee: Option<Coin>,
        new_feature_toggle: Option<FeatureToggle>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        result(self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
            &white_whale_std::pool_manager::ExecuteMsg::UpdateConfig {
                whale_lair_addr: new_whale_lair_addr.map(|addr| addr.to_string()),
                pool_creation_fee: new_pool_creation_fee,
                feature_toggle: new_feature_toggle,
            },
            &[],
        ));

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
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Ownership {},
            );

        result(ownership_response);

        self
    }

    pub(crate) fn query_balance(
        &mut self,
        addr: String,
        denom: impl Into<String>,
        result: impl Fn(StdResult<Coin>),
    ) -> &mut Self {
        let balance_resp: StdResult<Coin> = self.app.wrap().query_balance(addr, denom);

        result(balance_resp);

        self
    }

    pub(crate) fn query_all_balances(
        &mut self,
        addr: String,
        result: impl Fn(StdResult<Vec<Coin>>),
    ) -> &mut Self {
        let balance_resp: StdResult<Vec<Coin>> = self.app.wrap().query_all_balances(addr);

        result(balance_resp);

        self
    }

    pub(crate) fn _query_pair_info(
        &self,
        pair_identifier: String,
        result: impl Fn(StdResult<PairInfo>),
    ) -> &Self {
        let pair_info_response: StdResult<PairInfo> = self.app.wrap().query_wasm_smart(
            &self.pool_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::Pair { pair_identifier },
        );

        result(pair_info_response);

        self
    }

    pub(crate) fn query_simulation(
        &mut self,
        pair_identifier: String,
        offer_asset: Coin,
        ask_asset: String,
        result: impl Fn(StdResult<SimulationResponse>),
    ) -> &mut Self {
        let pair_info_response: StdResult<SimulationResponse> = self.app.wrap().query_wasm_smart(
            &self.pool_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::Simulation {
                offer_asset,
                ask_asset: Coin {
                    amount: Uint128::zero(),
                    denom: ask_asset,
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
        offer_asset: String,
        ask_asset: Coin,
        result: impl Fn(StdResult<ReverseSimulationResponse>),
    ) -> &mut Self {
        let pair_info_response: StdResult<ReverseSimulationResponse> =
            self.app.wrap().query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::ReverseSimulation {
                    offer_asset: Coin {
                        amount: Uint128::zero(),
                        denom: offer_asset,
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
        let lp_token_response: PairInfo = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Pair {
                    pair_identifier: identifier,
                },
            )
            .unwrap();

        // Get balance of LP token, if native we can just query balance otherwise we need to go to cw20

        let balance: Uint128 = self
            .app
            .wrap()
            .query_balance(sender, lp_token_response.lp_denom)
            .unwrap()
            .amount;

        result(Result::Ok(balance));
        self
    }

    pub(crate) fn _query_lp_token(&mut self, identifier: String, _sender: String) -> String {
        // Get the LP token from Config
        let lp_token_response: PairInfo = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Pair {
                    pair_identifier: identifier,
                },
            )
            .unwrap();

        // Get balance of LP token, if native we can just query balance otherwise we need to go to cw20
        lp_token_response.lp_denom
    }

    /// Retrieves the current configuration of the pool manager contract.
    pub(crate) fn query_config(&mut self) -> Config {
        self.app
            .wrap()
            .query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Config {},
            )
            .unwrap()
    }
}

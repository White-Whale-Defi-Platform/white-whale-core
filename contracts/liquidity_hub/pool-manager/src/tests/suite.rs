use cosmwasm_std::testing::MockStorage;
use std::cell::RefCell;
use white_whale_std::pool_manager::{
    Config, FeatureToggle, PoolInfoResponse, ReverseSimulateSwapOperationsResponse,
    ReverseSimulationResponse, SimulateSwapOperationsResponse, SimulationResponse, SwapOperation,
    SwapRoute, SwapRouteCreatorResponse, SwapRouteResponse, SwapRoutesResponse,
};
use white_whale_std::pool_manager::{InstantiateMsg, PoolType};

use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, StdResult, Timestamp, Uint128, Uint64};
use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, Contract, ContractWrapper, DistributionKeeper,
    Executor, FailingModule, GovFailingModule, IbcFailingModule, StakeKeeper, WasmKeeper,
};

use white_whale_std::epoch_manager::epoch_manager::{Epoch, EpochConfig};
use white_whale_std::fee::PoolFee;
use white_whale_std::incentive_manager::PositionsResponse;
use white_whale_std::lp_common::LP_SYMBOL;
use white_whale_testing::multi_test::stargate_mock::StargateMock;

fn contract_pool_manager() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);

    Box::new(contract)
}

/// Creates the whale lair contract
pub fn bonding_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        bonding_manager::contract::execute,
        bonding_manager::contract::instantiate,
        bonding_manager::contract::query,
    )
    .with_reply(bonding_manager::contract::reply)
    .with_migrate(bonding_manager::contract::migrate);

    Box::new(contract)
}

/// Creates the epoch manager contract
pub fn epoch_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        epoch_manager::contract::execute,
        epoch_manager::contract::instantiate,
        epoch_manager::contract::query,
    )
    .with_migrate(epoch_manager::contract::migrate);

    Box::new(contract)
}

/// Creates the incentive manager contract
pub fn incentive_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        incentive_manager::contract::execute,
        incentive_manager::contract::instantiate,
        incentive_manager::contract::query,
    )
    .with_migrate(incentive_manager::contract::migrate);

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
    pub bonding_manager_addr: Addr,
    pub pool_manager_addr: Addr,
    pub incentive_manager_addr: Addr,
    pub epoch_manager_addr: Addr,
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
    pub(crate) fn add_one_day(&mut self) -> &mut Self {
        let mut block_info = self.app.block_info();
        block_info.time = block_info.time.plus_days(1);
        self.app.set_block(block_info);

        self
    }

    pub(crate) fn get_lp_denom(&self, pool_identifier: String) -> String {
        format!(
            "factory/{}/u{}.pool.{}.{}",
            self.pool_manager_addr, pool_identifier, pool_identifier, LP_SYMBOL
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
            bonding_manager_addr: Addr::unchecked(""),
            pool_manager_addr: Addr::unchecked(""),
            incentive_manager_addr: Addr::unchecked(""),
            epoch_manager_addr: Addr::unchecked(""),
            cw20_tokens: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate(
        &mut self,
        bonding_manager_addr: String,
        incentive_manager_addr: String,
    ) -> &mut Self {
        let msg = InstantiateMsg {
            bonding_manager_addr,
            incentive_manager_addr,
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
                Some(creator.clone().into_string()),
            )
            .unwrap();

        let bonding_manager_addr = self.bonding_manager_addr.clone();

        if !bonding_manager_addr.into_string().is_empty() {
            let pool_manager_addr = self.pool_manager_addr.clone();
            let epoch_manager_addr = self.epoch_manager_addr.clone();

            let msg = white_whale_std::bonding_manager::ExecuteMsg::UpdateConfig {
                epoch_manager_addr: Some(epoch_manager_addr.into_string()),
                pool_manager_addr: Some(pool_manager_addr.into_string()),
                unbonding_period: None,
                growth_rate: None,
            };

            self.app
                .execute_contract(
                    creator.clone(),
                    self.bonding_manager_addr.clone(),
                    &msg,
                    &[],
                )
                .unwrap();
        }

        self
    }

    #[track_caller]
    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        self.create_epoch_manager();
        self.create_bonding_manager();
        self.create_incentive_manager();
        self.add_hook(self.incentive_manager_addr.clone());
        self.add_hook(self.bonding_manager_addr.clone());

        // 25 April 2024 15:00:00 UTC
        let timestamp = Timestamp::from_seconds(1714057200);
        self.set_time(timestamp);

        self.instantiate(
            self.bonding_manager_addr.to_string(),
            self.incentive_manager_addr.to_string(),
        )
    }

    fn create_bonding_manager(&mut self) {
        let bonding_manager_id = self.app.store_code(bonding_manager_contract());
        let epoch_manager_addr = self.epoch_manager_addr.to_string();

        let msg = white_whale_std::bonding_manager::InstantiateMsg {
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 86_400u64,
            growth_rate: Decimal::one(),
            bonding_assets: vec!["bWHALE".to_string(), "ampWHALE".to_string()],
            grace_period: Default::default(),
            epoch_manager_addr,
        };

        let creator = self.creator().clone();

        self.bonding_manager_addr = self
            .app
            .instantiate_contract(
                bonding_manager_id,
                creator.clone(),
                &msg,
                &[],
                "Bonding Manager".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }
    fn create_epoch_manager(&mut self) {
        let epoch_manager_id = self.app.store_code(epoch_manager_contract());

        let msg = white_whale_std::epoch_manager::epoch_manager::InstantiateMsg {
            start_epoch: Epoch {
                id: 0,
                start_time: Timestamp::from_seconds(1714057200),
            },
            epoch_config: EpochConfig {
                duration: Uint64::new(86_400_000000000),
                genesis_epoch: Uint64::new(1714057200_000000000),
            },
        };

        let creator = self.creator().clone();

        self.epoch_manager_addr = self
            .app
            .instantiate_contract(
                epoch_manager_id,
                creator.clone(),
                &msg,
                &[],
                "Epoch Manager".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }

    fn add_hook(&mut self, contract: Addr) {
        let msg = white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::AddHook {
            contract_addr: contract.to_string(),
        };

        let creator = self.creator().clone();

        self.app
            .execute_contract(creator, self.epoch_manager_addr.clone(), &msg, &[])
            .unwrap();
    }

    fn create_incentive_manager(&mut self) {
        let incentive_manager_id = self.app.store_code(incentive_manager_contract());

        let creator = self.creator().clone();
        let epoch_manager_addr = self.epoch_manager_addr.to_string();
        let bonding_manager_addr = self.bonding_manager_addr.to_string();

        let msg = white_whale_std::incentive_manager::InstantiateMsg {
            owner: creator.clone().to_string(),
            epoch_manager_addr,
            bonding_manager_addr,
            create_incentive_fee: Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::zero(),
            },
            max_concurrent_incentives: 5,
            max_incentive_epoch_buffer: 014,
            min_unlocking_duration: 86_400,
            max_unlocking_duration: 31_536_000,
            emergency_unlock_penalty: Decimal::percent(10),
        };

        self.incentive_manager_addr = self
            .app
            .instantiate_contract(
                incentive_manager_id,
                creator.clone(),
                &msg,
                &[],
                "Incentive Manager".to_string(),
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
        pool_identifier: String,
        unlocking_duration: Option<u64>,
        lock_position_identifier: Option<String>,
        max_spread: Option<Decimal>,
        receiver: Option<String>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::ProvideLiquidity {
            pool_identifier,
            slippage_tolerance: None,
            max_spread,
            receiver,
            unlocking_duration,
            lock_position_identifier,
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
        ask_asset_denom: String,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        receiver: Option<String>,
        pool_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::Swap {
            ask_asset_denom,
            belief_price,
            max_spread,
            receiver,
            pool_identifier,
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
        receiver: Option<String>,
        max_spread: Option<Decimal>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::ExecuteSwapOperations {
            operations,
            minimum_receive,
            receiver,
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
    pub(crate) fn create_pool(
        &mut self,
        sender: Addr,
        asset_denoms: Vec<String>,
        asset_decimals: Vec<u8>,
        pool_fees: PoolFee,
        pool_type: PoolType,
        pool_identifier: Option<String>,
        pool_creation_fee_funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::CreatePool {
            asset_denoms,
            asset_decimals,
            pool_fees,
            pool_type,
            pool_identifier,
        };

        result(self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
            &msg,
            &pool_creation_fee_funds,
        ));

        self
    }

    #[track_caller]
    pub(crate) fn withdraw_liquidity(
        &mut self,
        sender: Addr,
        pool_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::WithdrawLiquidity { pool_identifier };

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
        new_bonding_manager_addr: Option<Addr>,
        new_pool_creation_fee: Option<Coin>,
        new_feature_toggle: Option<FeatureToggle>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        result(self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
            &white_whale_std::pool_manager::ExecuteMsg::UpdateConfig {
                bonding_manager_addr: new_bonding_manager_addr.map(|addr| addr.to_string()),
                pool_creation_fee: new_pool_creation_fee,
                feature_toggle: new_feature_toggle,
            },
            &[],
        ));

        self
    }

    /// Adds swap routes to the pool manager contract.
    #[track_caller]
    pub(crate) fn add_swap_routes(
        &mut self,
        sender: Addr,
        swap_routes: Vec<SwapRoute>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        result(self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
            &white_whale_std::pool_manager::ExecuteMsg::AddSwapRoutes { swap_routes },
            &[],
        ));

        self
    }

    /// Removes swap routes from the pool manager contract.
    #[track_caller]
    pub(crate) fn remove_swap_routes(
        &mut self,
        sender: Addr,
        swap_routes: Vec<SwapRoute>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        result(self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
            &white_whale_std::pool_manager::ExecuteMsg::RemoveSwapRoutes { swap_routes },
            &[],
        ));

        self
    }

    /// Creates a new epoch.
    #[track_caller]
    pub(crate) fn create_new_epoch(
        &mut self,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let user = self.creator();

        result(self.app.execute_contract(
            user,
            self.epoch_manager_addr.clone(),
            &white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::CreateEpoch,
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

    pub(crate) fn query_pool_info(
        &self,
        pool_identifier: String,
        result: impl Fn(StdResult<PoolInfoResponse>),
    ) -> &Self {
        let pool_info_response: StdResult<PoolInfoResponse> = self.app.wrap().query_wasm_smart(
            &self.pool_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::Pool { pool_identifier },
        );

        result(pool_info_response);

        self
    }

    pub(crate) fn query_simulation(
        &mut self,
        pool_identifier: String,
        offer_asset: Coin,
        ask_asset_denom: String,
        result: impl Fn(StdResult<SimulationResponse>),
    ) -> &mut Self {
        let pool_info_response: StdResult<SimulationResponse> = self.app.wrap().query_wasm_smart(
            &self.pool_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::Simulation {
                offer_asset,
                ask_asset_denom,
                pool_identifier,
            },
        );

        result(pool_info_response);

        self
    }

    pub(crate) fn query_reverse_simulation(
        &mut self,
        pool_identifier: String,
        ask_asset: Coin,
        offer_asset_denom: String,
        result: impl Fn(StdResult<ReverseSimulationResponse>),
    ) -> &mut Self {
        let pool_info_response: StdResult<ReverseSimulationResponse> =
            self.app.wrap().query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::ReverseSimulation {
                    ask_asset,
                    offer_asset_denom,
                    pool_identifier,
                },
            );

        result(pool_info_response);

        self
    }

    pub(crate) fn query_simulate_swap_operations(
        &mut self,
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
        result: impl Fn(StdResult<SimulateSwapOperationsResponse>),
    ) -> &mut Self {
        let pool_info_response: StdResult<SimulateSwapOperationsResponse> =
            self.app.wrap().query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::SimulateSwapOperations {
                    offer_amount,
                    operations,
                },
            );

        result(pool_info_response);

        self
    }

    pub(crate) fn query_reverse_simulate_swap_operations(
        &mut self,
        ask_amount: Uint128,
        operations: Vec<SwapOperation>,
        result: impl Fn(StdResult<ReverseSimulateSwapOperationsResponse>),
    ) -> &mut Self {
        let pool_info_response: StdResult<ReverseSimulateSwapOperationsResponse> =
            self.app.wrap().query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::ReverseSimulateSwapOperations {
                    ask_amount,
                    operations,
                },
            );

        result(pool_info_response);

        self
    }

    pub(crate) fn query_amount_of_lp_token(
        &mut self,
        identifier: String,
        sender: String,
        result: impl Fn(StdResult<Uint128>),
    ) -> &mut Self {
        // Get the LP token from Config
        let lp_token_response: PoolInfoResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Pool {
                    pool_identifier: identifier,
                },
            )
            .unwrap();

        // Get balance of LP token, if native we can just query balance otherwise we need to go to cw20

        let balance: Uint128 = self
            .app
            .wrap()
            .query_balance(sender, lp_token_response.pool_info.lp_denom)
            .unwrap()
            .amount;

        result(Result::Ok(balance));
        self
    }

    /// Retrieves the current configuration of the pool manager contract.
    pub(crate) fn query_config(&mut self) -> Config {
        self.app
            .wrap()
            .query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::Config,
            )
            .unwrap()
    }

    /// Retrieves a swap route for a given pool of assets.
    pub(crate) fn query_swap_route(
        &mut self,
        offer_asset_denom: String,
        ask_asset_denom: String,
        result: impl Fn(StdResult<SwapRouteResponse>),
    ) -> &mut Self {
        let swap_route_response: StdResult<SwapRouteResponse> = self.app.wrap().query_wasm_smart(
            &self.pool_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::SwapRoute {
                offer_asset_denom,
                ask_asset_denom,
            },
        );

        result(swap_route_response);

        self
    }

    /// Retrieves the swap routes for a given poolr of assets.
    pub(crate) fn query_swap_routes(
        &mut self,
        result: impl Fn(StdResult<SwapRoutesResponse>),
    ) -> &mut Self {
        let swap_routes_response: StdResult<SwapRoutesResponse> = self.app.wrap().query_wasm_smart(
            &self.pool_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::SwapRoutes,
        );

        result(swap_routes_response);

        self
    }

    /// Retrieves the swap route creator for a given pool of assets.
    pub(crate) fn query_swap_route_creator(
        &mut self,
        offer_asset_denom: String,
        ask_asset_denom: String,
        result: impl Fn(StdResult<SwapRouteCreatorResponse>),
    ) -> &mut Self {
        let swap_route_creator_response: StdResult<SwapRouteCreatorResponse> =
            self.app.wrap().query_wasm_smart(
                &self.pool_manager_addr,
                &white_whale_std::pool_manager::QueryMsg::SwapRouteCreator {
                    offer_asset_denom,
                    ask_asset_denom,
                },
            );

        result(swap_route_creator_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_incentive_positions(
        &mut self,
        address: Addr,
        open_state: Option<bool>,
        result: impl Fn(StdResult<PositionsResponse>),
    ) -> &mut Self {
        let positions_response: StdResult<PositionsResponse> = self.app.wrap().query_wasm_smart(
            &self.incentive_manager_addr,
            &white_whale_std::incentive_manager::QueryMsg::Positions {
                address: address.to_string(),
                open_state,
            },
        );

        result(positions_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_lp_supply(
        &mut self,
        identifier: String,
        result: impl Fn(StdResult<Coin>),
    ) -> &mut Self {
        let lp_denom = RefCell::new("".to_string());

        self.query_pool_info(identifier.clone(), |res| {
            let response = res.unwrap();
            *lp_denom.borrow_mut() = response.pool_info.lp_denom.clone();
        });

        let supply_response = self.app.wrap().query_supply(lp_denom.into_inner());

        result(supply_response);

        self
    }
}

use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, StdResult, Timestamp, Uint128, Uint64};
use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, DistributionKeeper, Executor, FailingModule,
    GovFailingModule, IbcFailingModule, StakeKeeper, WasmKeeper,
};

use white_whale_std::epoch_manager::epoch_manager::{Epoch, EpochConfig, EpochResponse};
use white_whale_std::incentive_manager::{
    Config, IncentiveAction, IncentivesBy, IncentivesResponse, InstantiateMsg, LpWeightResponse,
    PositionAction, PositionsResponse, RewardsResponse,
};
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairType};
use white_whale_std::pool_network::pair::ExecuteMsg::ProvideLiquidity;
use white_whale_std::pool_network::pair::{PoolFee, SimulationResponse};
use white_whale_testing::multi_test::stargate_mock::StargateMock;

use crate::common::suite_contracts::{
    epoch_manager_contract, incentive_manager_contract, whale_lair_contract,
};
use crate::common::MOCK_CONTRACT_ADDR;

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
    pub incentive_manager_addr: Addr,
    pub whale_lair_addr: Addr,
    pub epoch_manager_addr: Addr,
    pub pools: Vec<Addr>,
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
            incentive_manager_addr: Addr::unchecked(""),
            whale_lair_addr: Addr::unchecked(""),
            epoch_manager_addr: Addr::unchecked(""),
            pools: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        self.create_whale_lair();
        self.create_epoch_manager();

        // April 4th 2024 15:00:00 UTC
        let timestamp = Timestamp::from_seconds(1712242800u64);
        self.set_time(timestamp);

        // instantiates the incentive manager contract
        self.instantiate(
            self.whale_lair_addr.to_string(),
            self.epoch_manager_addr.to_string(),
            Coin {
                denom: "uwhale".to_string(),
                amount: Uint128::new(1_000u128),
            },
            2,
            14,
            86_400,
            31_536_000,
            Decimal::percent(10), //10% penalty
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

    fn create_epoch_manager(&mut self) {
        let epoch_manager_contract = self.app.store_code(epoch_manager_contract());

        // create epoch manager
        let msg = white_whale_std::epoch_manager::epoch_manager::InstantiateMsg {
            start_epoch: Epoch {
                id: 10,
                start_time: Timestamp::from_nanos(1712242800_000000000u64),
            },
            epoch_config: EpochConfig {
                duration: Uint64::new(86400_000000000u64),
                genesis_epoch: Uint64::new(1712242800_000000000u64), // April 4th 2024 15:00:00 UTC
            },
        };

        let creator = self.creator().clone();

        self.epoch_manager_addr = self
            .app
            .instantiate_contract(
                epoch_manager_contract,
                creator.clone(),
                &msg,
                &[],
                "Epoch Manager".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }

    #[track_caller]
    pub(crate) fn instantiate(
        &mut self,
        whale_lair_addr: String,
        epoch_manager_addr: String,
        create_incentive_fee: Coin,
        max_concurrent_incentives: u32,
        max_incentive_epoch_buffer: u32,
        min_unlocking_duration: u64,
        max_unlocking_duration: u64,
        emergency_unlock_penalty: Decimal,
    ) -> &mut Self {
        let msg = InstantiateMsg {
            owner: self.creator().to_string(),
            epoch_manager_addr,
            whale_lair_addr,
            create_incentive_fee,
            max_concurrent_incentives,
            max_incentive_epoch_buffer,
            min_unlocking_duration,
            max_unlocking_duration,
            emergency_unlock_penalty,
        };

        let incentive_manager_id = self.app.store_code(incentive_manager_contract());

        let creator = self.creator().clone();

        self.incentive_manager_addr = self
            .app
            .instantiate_contract(
                incentive_manager_id,
                creator.clone(),
                &msg,
                &[],
                "WW Incentive Manager",
                Some(creator.into_string()),
            )
            .unwrap();
        self
    }

    #[track_caller]
    pub(crate) fn instantiate_err(
        &mut self,
        whale_lair_addr: String,
        epoch_manager_addr: String,
        create_incentive_fee: Coin,
        max_concurrent_incentives: u32,
        max_incentive_epoch_buffer: u32,
        min_unlocking_duration: u64,
        max_unlocking_duration: u64,
        emergency_unlock_penalty: Decimal,
        result: impl Fn(anyhow::Result<Addr>),
    ) -> &mut Self {
        let msg = InstantiateMsg {
            owner: self.creator().to_string(),
            epoch_manager_addr,
            whale_lair_addr,
            create_incentive_fee,
            max_concurrent_incentives,
            max_incentive_epoch_buffer,
            min_unlocking_duration,
            max_unlocking_duration,
            emergency_unlock_penalty,
        };

        let incentive_manager_id = self.app.store_code(incentive_manager_contract());

        let creator = self.creator().clone();

        result(self.app.instantiate_contract(
            incentive_manager_id,
            creator.clone(),
            &msg,
            &[],
            "WW Incentive Manager",
            Some(creator.into_string()),
        ));

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
        let msg = white_whale_std::incentive_manager::ExecuteMsg::UpdateOwnership(action);

        result(
            self.app
                .execute_contract(sender, self.incentive_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn update_config(
        &mut self,
        sender: Addr,
        whale_lair_addr: Option<String>,
        epoch_manager_addr: Option<String>,
        create_incentive_fee: Option<Coin>,
        max_concurrent_incentives: Option<u32>,
        max_incentive_epoch_buffer: Option<u32>,
        min_unlocking_duration: Option<u64>,
        max_unlocking_duration: Option<u64>,
        emergency_unlock_penalty: Option<Decimal>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::incentive_manager::ExecuteMsg::UpdateConfig {
            whale_lair_addr,
            epoch_manager_addr,
            create_incentive_fee,
            max_concurrent_incentives,
            max_incentive_epoch_buffer,
            min_unlocking_duration,
            max_unlocking_duration,
            emergency_unlock_penalty,
        };

        result(self.app.execute_contract(
            sender,
            self.incentive_manager_addr.clone(),
            &msg,
            &funds,
        ));

        self
    }

    #[track_caller]
    pub(crate) fn manage_incentive(
        &mut self,
        sender: Addr,
        action: IncentiveAction,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::incentive_manager::ExecuteMsg::ManageIncentive { action };

        result(self.app.execute_contract(
            sender,
            self.incentive_manager_addr.clone(),
            &msg,
            &funds,
        ));

        self
    }

    #[track_caller]
    pub(crate) fn manage_position(
        &mut self,
        sender: Addr,
        action: PositionAction,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::incentive_manager::ExecuteMsg::ManagePosition { action };

        result(self.app.execute_contract(
            sender,
            self.incentive_manager_addr.clone(),
            &msg,
            &funds,
        ));

        self
    }

    #[track_caller]
    pub(crate) fn claim(
        &mut self,
        sender: Addr,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::incentive_manager::ExecuteMsg::Claim;

        result(self.app.execute_contract(
            sender,
            self.incentive_manager_addr.clone(),
            &msg,
            &funds,
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
                &self.incentive_manager_addr,
                &white_whale_std::incentive_manager::QueryMsg::Ownership {},
            );

        result(ownership_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
        let response: StdResult<Config> = self.app.wrap().query_wasm_smart(
            &self.incentive_manager_addr,
            &white_whale_std::incentive_manager::QueryMsg::Config {},
        );

        result(response);

        self
    }

    #[track_caller]
    pub(crate) fn query_incentives(
        &mut self,
        filter_by: Option<IncentivesBy>,
        start_after: Option<String>,
        limit: Option<u32>,
        result: impl Fn(StdResult<IncentivesResponse>),
    ) -> &mut Self {
        let incentives_response: StdResult<IncentivesResponse> = self.app.wrap().query_wasm_smart(
            &self.incentive_manager_addr,
            &white_whale_std::incentive_manager::QueryMsg::Incentives {
                filter_by,
                start_after,
                limit,
            },
        );

        result(incentives_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_positions(
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
    pub(crate) fn query_rewards(
        &mut self,
        address: Addr,
        result: impl Fn(StdResult<RewardsResponse>),
    ) -> &mut Self {
        let rewards_response: StdResult<RewardsResponse> = self.app.wrap().query_wasm_smart(
            &self.incentive_manager_addr,
            &white_whale_std::incentive_manager::QueryMsg::Rewards {
                address: address.to_string(),
            },
        );

        result(rewards_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_lp_weight(
        &mut self,
        denom: &str,
        epoch_id: u64,
        result: impl Fn(StdResult<LpWeightResponse>),
    ) -> &mut Self {
        let rewards_response: StdResult<LpWeightResponse> = self.app.wrap().query_wasm_smart(
            &self.incentive_manager_addr,
            &white_whale_std::incentive_manager::QueryMsg::LPWeight {
                denom: denom.to_string(),
                epoch_id,
            },
        );

        result(rewards_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_balance(
        &mut self,
        denom: String,
        address: Addr,
        result: impl Fn(Uint128),
    ) -> &mut Self {
        let balance_response = self.app.wrap().query_balance(address, denom.clone());
        result(balance_response.unwrap_or(coin(0, denom)).amount);

        self
    }
}

/// Epoch manager actions
impl TestingSuite {
    #[track_caller]
    pub(crate) fn create_epoch(
        &mut self,
        sender: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::CreateEpoch {};

        result(
            self.app
                .execute_contract(sender, self.epoch_manager_addr.clone(), &msg, &vec![]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn add_hook(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::AddHook {
            contract_addr: contract_addr.to_string(),
        };

        result(
            self.app
                .execute_contract(sender, self.epoch_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn remove_hook(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::RemoveHook {
            contract_addr: contract_addr.to_string(),
        };

        result(
            self.app
                .execute_contract(sender, self.epoch_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn query_current_epoch(
        &mut self,
        result: impl Fn(StdResult<EpochResponse>),
    ) -> &mut Self {
        let current_epoch_response: StdResult<EpochResponse> = self.app.wrap().query_wasm_smart(
            &self.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        );

        result(current_epoch_response);

        self
    }
}

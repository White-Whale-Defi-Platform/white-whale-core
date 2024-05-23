use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, StdResult, Timestamp, Uint64};
use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, DistributionKeeper, Executor, FailingModule,
    GovFailingModule, IbcFailingModule, StakeKeeper, WasmKeeper,
};

use white_whale_std::epoch_manager::epoch_manager::{Epoch, EpochConfig, EpochResponse};
use white_whale_testing::multi_test::stargate_mock::StargateMock;

use crate::common::suite_contracts::{
    bonding_manager_contract, epoch_manager_contract, incentive_manager_contract,
    pool_manager_contract, vault_manager_contract,
};

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
    pub senders: [Addr; 5],
    pub bonding_manager_addr: Addr,
    pub epoch_manager_addr: Addr,
    pub incentive_manager_addr: Addr,
    pub pool_manager_addr: Addr,
    pub vault_manager_addr: Addr,
    pub pools: Vec<Addr>,
    pub vaults: Vec<Addr>,
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

    pub(crate) fn add_one_epoch(&mut self) -> &mut Self {
        let creator = self.creator();

        self.add_one_day().create_epoch(creator, |res| {
            res.unwrap();
        });

        self
    }
}

/// Instantiate
impl TestingSuite {
    pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
        let sender_1 = Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3");
        let sender_2 = Addr::unchecked("migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40");
        let sender_3 = Addr::unchecked("migaloo1ludaslnu24p5eftw499f7ngsc2jkzqdsrvxt75");
        let sender_4 = Addr::unchecked("migaloo1lh7mmdavky83xks76ch57whjaqa7e456vvpz8y");
        let sender_5 = Addr::unchecked("migaloo13y3petsaw4vfchac4frjmuuevjjjcceja7sjx7");

        let bank = BankKeeper::new();

        let balances = vec![
            (sender_1.clone(), initial_balance.clone()),
            (sender_2.clone(), initial_balance.clone()),
            (sender_3.clone(), initial_balance.clone()),
            (sender_4.clone(), initial_balance.clone()),
            (sender_5.clone(), initial_balance.clone()),
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
            senders: [sender_1, sender_2, sender_3, sender_4, sender_5],
            bonding_manager_addr: Addr::unchecked(""),
            epoch_manager_addr: Addr::unchecked(""),
            incentive_manager_addr: Addr::unchecked(""),
            pool_manager_addr: Addr::unchecked(""),
            vault_manager_addr: Addr::unchecked(""),
            pools: vec![],
            vaults: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate(&mut self) -> &mut Self {
        self.create_epoch_manager();
        self.create_bonding_manager();
        self.create_incentive_manager();
        self.create_pool_manager();
        self.create_vault_manager();

        // May 23th 2024 15:00:00 UTC
        let timestamp = Timestamp::from_seconds(1716476400u64);
        self.set_time(timestamp);

        self.add_one_epoch();

        self
    }

    fn create_bonding_manager(&mut self) {
        let bonding_manager_id = self.app.store_code(bonding_manager_contract());
        let epoch_manager_addr = self.epoch_manager_addr.to_string();

        // create whale lair
        let msg = white_whale_std::bonding_manager::InstantiateMsg {
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 1u64,
            growth_rate: Decimal::one(),
            bonding_assets: vec!["ampWHALE".to_string(), "bWHALE".to_string()],
            grace_period: 21,
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
                "Migaloo Bonding Manager".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }

    #[allow(clippy::inconsistent_digit_grouping)]
    fn create_epoch_manager(&mut self) {
        let epoch_manager_contract = self.app.store_code(epoch_manager_contract());

        // create epoch manager
        let msg = white_whale_std::epoch_manager::epoch_manager::InstantiateMsg {
            start_epoch: Epoch {
                id: 0,
                start_time: Timestamp::from_nanos(1716476400_000000000u64),
            },
            epoch_config: EpochConfig {
                duration: Uint64::new(86400_000000000u64),
                genesis_epoch: Uint64::new(1716476400_000000000u64), // May 23th 2024 15:00:00 UTC
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

    fn create_incentive_manager(&mut self) {
        let incentive_manager_contract = self.app.store_code(incentive_manager_contract());

        let epoch_manager_addr = self.epoch_manager_addr.to_string();
        let bonding_manager_addr = self.bonding_manager_addr.to_string();

        let creator = self.creator().clone();

        // create epoch manager
        let msg = white_whale_std::incentive_manager::InstantiateMsg {
            owner: creator.to_string(),
            epoch_manager_addr,
            bonding_manager_addr,
            create_incentive_fee: coin(1_000, "uwhale"),
            max_concurrent_incentives: 5,
            max_incentive_epoch_buffer: 14,
            min_unlocking_duration: 86_400,
            max_unlocking_duration: 31_536_000,
            emergency_unlock_penalty: Decimal::percent(10),
        };

        self.incentive_manager_addr = self
            .app
            .instantiate_contract(
                incentive_manager_contract,
                creator.clone(),
                &msg,
                &[],
                "Incentive Manager".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }
    fn create_pool_manager(&mut self) {
        let pool_manager_contract = self.app.store_code(pool_manager_contract());

        let bonding_manager_addr = self.bonding_manager_addr.to_string();
        let incentive_manager_addr = self.incentive_manager_addr.to_string();

        // create epoch manager
        let msg = white_whale_std::pool_manager::InstantiateMsg {
            bonding_manager_addr,
            incentive_manager_addr,
            pool_creation_fee: coin(1_000, "uwhale"),
        };

        let creator = self.creator().clone();

        self.pool_manager_addr = self
            .app
            .instantiate_contract(
                pool_manager_contract,
                creator.clone(),
                &msg,
                &[],
                "Pool Manager".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }
    fn create_vault_manager(&mut self) {
        let vault_manager_contract = self.app.store_code(vault_manager_contract());

        let creator = self.creator().clone();
        let bonding_manager_addr = self.bonding_manager_addr.to_string();

        // create epoch manager
        let msg = white_whale_std::vault_manager::InstantiateMsg {
            owner: creator.to_string(),
            bonding_manager_addr,
            vault_creation_fee: coin(1_000, "uwhale"),
        };

        self.vault_manager_addr = self
            .app
            .instantiate_contract(
                vault_manager_contract,
                creator.clone(),
                &msg,
                &[],
                "Vault Manager".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }
}

//------------------------------------//

/// bonding manager actions
impl TestingSuite {}

//------------------------------------//

/// epoch manager actions
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
                .execute_contract(sender, self.epoch_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn add_hook(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::AddHook {
            contract_addr: contract_addr.to_string(),
        };

        result(
            self.app
                .execute_contract(sender, self.epoch_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn remove_hook(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::RemoveHook {
            contract_addr: contract_addr.to_string(),
        };

        result(
            self.app
                .execute_contract(sender, self.epoch_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn query_current_epoch(
        &mut self,
        mut result: impl FnMut(StdResult<EpochResponse>),
    ) -> &mut Self {
        let current_epoch_response: StdResult<EpochResponse> = self.app.wrap().query_wasm_smart(
            &self.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::CurrentEpoch {},
        );

        result(current_epoch_response);

        self
    }
    #[track_caller]
    pub(crate) fn query_hooks(
        &mut self,
        mut result: impl FnMut(StdResult<EpochResponse>),
    ) -> &mut Self {
        let current_epoch_response: StdResult<EpochResponse> = self.app.wrap().query_wasm_smart(
            &self.epoch_manager_addr,
            &white_whale_std::epoch_manager::epoch_manager::QueryMsg::Hooks {},
        );

        result(current_epoch_response);

        self
    }
}

//------------------------------------//

/// incentive manager actions
impl TestingSuite {}

//------------------------------------//

/// pool manager actions
impl TestingSuite {}

//------------------------------------//

/// vault manager actions
impl TestingSuite {}

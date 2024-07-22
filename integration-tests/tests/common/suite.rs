use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{
    coin, Addr, Coin, CosmosMsg, Decimal, Empty, StdResult, Timestamp, Uint128, Uint64,
};
use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, DistributionKeeper, Executor, FailingModule,
    GovFailingModule, IbcFailingModule, StakeKeeper, WasmKeeper,
};

use white_whale_std::bonding_manager::{
    BondedResponse, ClaimableRewardBucketsResponse, ExecuteMsg, GlobalIndex, QueryMsg,
    RewardBucket, RewardsResponse, UnbondingResponse, WithdrawableResponse,
};
use white_whale_std::epoch_manager::epoch_manager::{Epoch, EpochConfig, EpochResponse};
use white_whale_std::fee::PoolFee;
use white_whale_std::incentive_manager::{
    IncentiveAction, IncentivesBy, IncentivesResponse, LpWeightResponse, PositionAction,
    PositionsResponse,
};
use white_whale_std::pool_manager::{
    PoolType, PoolsResponse, ReverseSimulateSwapOperationsResponse, ReverseSimulationResponse,
    SimulateSwapOperationsResponse, SimulationResponse, SwapOperation, SwapRoute,
    SwapRoutesResponse,
};
use white_whale_std::vault_manager::{
    FilterVaultBy, PaybackAssetResponse, ShareResponse, VaultsResponse,
};
use white_whale_testing::multi_test::stargate_mock::StargateMock;

use crate::common::suite_contracts::{
    bonding_manager_contract, epoch_manager_contract, incentive_manager_contract,
    pool_manager_contract, vault_manager_contract,
};

pub const BWHALE: &str = "factory/migaloo1ludaslnu24p5eftw499f7ngsc2jkzqdsrvxt75/bWHALE";
pub const AMPWHALE: &str = "factory/migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40/ampWHALE";

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
    pub pool_identifiers: Vec<String>,
    pub vault_identifiers: Vec<String>,
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

    pub(crate) fn add_epochs(&mut self, n: u64) -> &mut Self {
        for _ in 0..n {
            self.add_one_epoch();
        }

        self
    }

    #[track_caller]
    pub(crate) fn _query_balance(
        &mut self,
        denom: String,
        address: Addr,
        result: impl Fn(Uint128),
    ) -> &mut Self {
        let balance_response = self.app.wrap().query_balance(address, denom.clone());
        result(balance_response.unwrap_or(coin(0, denom)).amount);

        self
    }

    pub(crate) fn _query_all_balances(
        &mut self,
        addr: String,
        result: impl Fn(StdResult<Vec<Coin>>),
    ) -> &mut Self {
        let balance_resp: StdResult<Vec<Coin>> = self.app.wrap().query_all_balances(addr);

        result(balance_resp);

        self
    }
}

/// Instantiate
impl TestingSuite {
    pub(crate) fn default_with_balances() -> Self {
        let initial_balance = vec![
            coin(1_000_000_000_000u128, "uwhale"),
            coin(1_000_000_000_000u128, "uosmo"),
            coin(1_000_000_000_000u128, "uusdc"),
            coin(1_000_000_000_000u128, "uusdt"),
            // ibc token is stablecoin
            coin(
                1_000_000_000_000u128,
                "ibc/BEFB9AB13AB43157A0AF6254AD4B1F565AC0CA0C1760B8339BE7B9E2996F7752",
            ),
            coin(1_000_000_000_000u128, AMPWHALE),
            coin(1_000_000_000_000u128, BWHALE),
            coin(
                1_000_000_000_000u128,
                "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5",
            ),
            coin(1_000_000_000_000_000u128, "btc"),
            coin(1_000_000_000_000_000_000_000_000u128, "inj"),
        ];

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
            pool_identifiers: vec![],
            vault_identifiers: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate(&mut self) -> &mut Self {
        let creator = self.creator().clone();

        self.create_epoch_manager();
        self.create_bonding_manager();
        self.create_incentive_manager();
        self.create_pool_manager();
        self.create_vault_manager();

        self.update_bonding_manager_contract_addresses(creator.clone(), |response| {
            response.unwrap();
        });

        let bonding_manager_addr = self.bonding_manager_addr.clone();
        let incentive_manager_addr = self.incentive_manager_addr.clone();

        self.add_hook(creator.clone(), bonding_manager_addr, |result| {
            result.unwrap();
        });

        self.add_hook(creator, incentive_manager_addr, |result| {
            result.unwrap();
        });

        // May 23th 2024 15:00:00 UTC
        let timestamp = Timestamp::from_seconds(1716476400u64);
        self.set_time(timestamp);

        self.add_one_epoch();

        self
    }

    fn create_bonding_manager(&mut self) {
        let bonding_manager_id = self.app.store_code(bonding_manager_contract());
        let epoch_manager_addr = self.epoch_manager_addr.to_string();

        let msg = white_whale_std::bonding_manager::InstantiateMsg {
            distribution_denom: "uwhale".to_string(),
            unbonding_period: 1u64,
            growth_rate: Decimal::one(),
            bonding_assets: vec![AMPWHALE.to_string(), BWHALE.to_string()],
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
impl TestingSuite {
    #[track_caller]
    pub(crate) fn bond(
        &mut self,
        sender: &Addr,
        funds: &[Coin],
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Bond;

        response(self.app.execute_contract(
            sender.clone(),
            self.bonding_manager_addr.clone(),
            &msg,
            funds,
        ));

        self
    }

    #[track_caller]
    pub(crate) fn unbond(
        &mut self,
        sender: Addr,
        asset: Coin,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Unbond { asset };

        response(
            self.app
                .execute_contract(sender, self.bonding_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn claim_bonding_rewards(
        &mut self,
        sender: &Addr,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Claim {};

        response(self.app.execute_contract(
            sender.clone(),
            self.bonding_manager_addr.clone(),
            &msg,
            &[],
        ));

        self
    }

    #[track_caller]
    pub(crate) fn withdraw_after_unbond(
        &mut self,
        sender: Addr,
        denom: String,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Withdraw { denom };

        response(
            self.app
                .execute_contract(sender, self.bonding_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn update_bonding_manager_contract_addresses(
        &mut self,
        sender: Addr,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let epoch_manager_addr = self.epoch_manager_addr.to_string();
        let pool_manager_addr = self.pool_manager_addr.to_string();

        let msg = ExecuteMsg::UpdateConfig {
            epoch_manager_addr: Some(epoch_manager_addr),
            pool_manager_addr: Some(pool_manager_addr),
            unbonding_period: None,
            growth_rate: None,
        };

        response(
            self.app
                .execute_contract(sender, self.bonding_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn _query_global_index(
        &mut self,
        reward_bucket_id: Option<u64>,
        response: impl Fn(StdResult<(&mut Self, GlobalIndex)>),
    ) -> &mut Self {
        let global_index: GlobalIndex = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.bonding_manager_addr,
                &QueryMsg::GlobalIndex { reward_bucket_id },
            )
            .unwrap();

        response(Ok((self, global_index)));

        self
    }

    #[track_caller]
    pub(crate) fn query_claimable_reward_buckets(
        &mut self,
        address: Option<&Addr>,
        response: impl Fn(StdResult<(&mut Self, Vec<RewardBucket>)>),
    ) -> &mut Self {
        let address = if let Some(address) = address {
            Some(address.to_string())
        } else {
            None
        };

        let query_res: ClaimableRewardBucketsResponse = self
            .app
            .wrap()
            .query_wasm_smart(&self.bonding_manager_addr, &QueryMsg::Claimable { address })
            .unwrap();

        response(Ok((self, query_res.reward_buckets)));

        self
    }

    #[track_caller]
    pub(crate) fn query_bonded(
        &mut self,
        address: Option<String>,
        response: impl Fn(StdResult<(&mut Self, BondedResponse)>),
    ) -> &mut Self {
        let bonded_response: BondedResponse = self
            .app
            .wrap()
            .query_wasm_smart(&self.bonding_manager_addr, &QueryMsg::Bonded { address })
            .unwrap();

        response(Ok((self, bonded_response)));

        self
    }

    #[track_caller]
    pub(crate) fn _query_unbonding(
        &mut self,
        address: String,
        denom: String,
        start_after: Option<u64>,
        limit: Option<u8>,
        response: impl Fn(StdResult<(&mut Self, UnbondingResponse)>),
    ) -> &mut Self {
        let unbonding_response: UnbondingResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.bonding_manager_addr,
                &QueryMsg::Unbonding {
                    address,
                    denom,
                    start_after,
                    limit,
                },
            )
            .unwrap();

        response(Ok((self, unbonding_response)));

        self
    }

    #[track_caller]
    pub(crate) fn _query_withdrawable(
        &mut self,
        address: String,
        denom: String,
        response: impl Fn(StdResult<(&mut Self, WithdrawableResponse)>),
    ) -> &mut Self {
        let withdrawable_response: WithdrawableResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.bonding_manager_addr,
                &QueryMsg::Withdrawable { address, denom },
            )
            .unwrap();

        response(Ok((self, withdrawable_response)));

        self
    }

    #[track_caller]
    pub(crate) fn query_bonding_rewards(
        &mut self,
        address: String,
        response: impl Fn(StdResult<(&mut Self, RewardsResponse)>),
    ) -> &mut Self {
        let rewards_response: RewardsResponse = self
            .app
            .wrap()
            .query_wasm_smart(&self.bonding_manager_addr, &QueryMsg::Rewards { address })
            .unwrap();

        response(Ok((self, rewards_response)));

        self
    }
}

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
    pub(crate) fn _remove_hook(
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
    pub(crate) fn _query_current_time(
        &mut self,
        mut result: impl FnMut(StdResult<Timestamp>),
    ) -> &mut Self {
        let current_time_response: StdResult<Timestamp> = Ok(self.app.block_info().time);

        result(current_time_response);

        self
    }

    #[track_caller]
    pub(crate) fn _query_hooks(
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
impl TestingSuite {
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
    pub(crate) fn _manage_position(
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
    pub(crate) fn _claim_incentive_rewards(
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

    #[track_caller]
    pub(crate) fn _query_incentives(
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
    pub(crate) fn _query_positions(
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
    pub(crate) fn _query_incentive_rewards(
        &mut self,
        address: Addr,
        result: impl Fn(StdResult<white_whale_std::incentive_manager::RewardsResponse>),
    ) -> &mut Self {
        let rewards_response: StdResult<white_whale_std::incentive_manager::RewardsResponse> =
            self.app.wrap().query_wasm_smart(
                &self.incentive_manager_addr,
                &white_whale_std::incentive_manager::QueryMsg::Rewards {
                    address: address.to_string(),
                },
            );

        result(rewards_response);

        self
    }

    #[track_caller]
    pub(crate) fn _query_lp_weight(
        &mut self,
        denom: &str,
        epoch_id: u64,
        result: impl Fn(StdResult<LpWeightResponse>),
    ) -> &mut Self {
        let rewards_response: StdResult<LpWeightResponse> = self.app.wrap().query_wasm_smart(
            &self.incentive_manager_addr,
            &white_whale_std::incentive_manager::QueryMsg::LPWeight {
                address: self.incentive_manager_addr.to_string(),
                denom: denom.to_string(),
                epoch_id,
            },
        );

        result(rewards_response);

        self
    }
}

//------------------------------------//

/// pool manager actions
impl TestingSuite {
    #[track_caller]
    pub(crate) fn provide_liquidity(
        &mut self,
        sender: &Addr,
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

        result(self.app.execute_contract(
            sender.clone(),
            self.pool_manager_addr.clone(),
            &msg,
            &funds,
        ));

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
    pub(crate) fn _execute_swap_operations(
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
        asset_denoms: Vec<&str>,
        asset_decimals: Vec<u8>,
        pool_fees: PoolFee,
        pool_type: PoolType,
        pool_identifier: Option<String>,
        pool_creation_fee_funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::CreatePool {
            asset_denoms: asset_denoms.iter().map(|&s| s.to_string()).collect(),
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
    pub(crate) fn _withdraw_liquidity(
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
    pub(crate) fn _remove_swap_routes(
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

    pub(crate) fn _query_pools(
        &self,
        pool_identifier: Option<String>,
        start_after: Option<String>,
        limit: Option<u32>,
        result: impl Fn(StdResult<PoolsResponse>),
    ) -> &Self {
        let pools_response: StdResult<PoolsResponse> = self.app.wrap().query_wasm_smart(
            &self.pool_manager_addr,
            &white_whale_std::pool_manager::QueryMsg::Pools {
                pool_identifier,
                start_after,
                limit,
            },
        );

        result(pools_response);

        self
    }

    pub(crate) fn _query_simulation(
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

    pub(crate) fn _query_reverse_simulation(
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

    pub(crate) fn _query_simulate_swap_operations(
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

    pub(crate) fn _query_reverse_simulate_swap_operations(
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

    /// Retrieves the swap routes for a given pool of assets.
    pub(crate) fn _query_swap_routes(
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
}

//------------------------------------//

/// vault manager actions
impl TestingSuite {
    #[track_caller]
    pub(crate) fn create_vault(
        &mut self,
        sender: Addr,
        asset_denom: &str,
        vault_identifier: Option<String>,
        fees: white_whale_std::vault_manager::VaultFee,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::vault_manager::ExecuteMsg::CreateVault {
            asset_denom: asset_denom.to_string(),
            fees,
            vault_identifier,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn _vault_deposit(
        &mut self,
        sender: Addr,
        vault_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::vault_manager::ExecuteMsg::Deposit { vault_identifier };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn _vault_withdraw(
        &mut self,
        sender: Addr,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::vault_manager::ExecuteMsg::Withdraw {};
        let vault_manager = self.vault_manager_addr.clone();

        result(
            self.app
                .execute_contract(sender, vault_manager, &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn _flashloan(
        &mut self,
        sender: Addr,
        asset: Coin,
        vault_identifier: String,
        payload: Vec<CosmosMsg>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::vault_manager::ExecuteMsg::FlashLoan {
            asset,
            vault_identifier,
            payload,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn _query_vault(
        &mut self,
        filter_by: FilterVaultBy,
        result: impl Fn(StdResult<VaultsResponse>),
    ) -> &mut Self {
        let vaults_response: StdResult<VaultsResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::vault_manager::QueryMsg::Vault { filter_by },
        );

        result(vaults_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_vaults(
        &mut self,
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
        result: impl Fn(StdResult<VaultsResponse>),
    ) -> &mut Self {
        let vaults_response: StdResult<VaultsResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::vault_manager::QueryMsg::Vaults { start_after, limit },
        );

        result(vaults_response);

        self
    }

    #[track_caller]
    pub(crate) fn _query_vault_share(
        &mut self,
        lp_share: Coin,
        result: impl Fn(StdResult<ShareResponse>),
    ) -> &mut Self {
        let response: StdResult<ShareResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::vault_manager::QueryMsg::Share { lp_share },
        );

        result(response);

        self
    }

    #[track_caller]
    pub(crate) fn _query_flashloan_payback(
        &mut self,
        asset: Coin,
        vault_identifier: String,
        result: impl Fn(StdResult<PaybackAssetResponse>),
    ) -> &mut Self {
        let response: StdResult<PaybackAssetResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::vault_manager::QueryMsg::PaybackAmount {
                asset,
                vault_identifier,
            },
        );

        result(response);

        self
    }
}

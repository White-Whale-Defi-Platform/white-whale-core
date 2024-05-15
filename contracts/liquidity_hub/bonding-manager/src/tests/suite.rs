use anyhow::Error;
use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    coin, from_json, Addr, Binary, Coin, Decimal, Empty, OwnedDeps, StdResult, Uint128, Uint64,
};
// use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, DistributionKeeper, Executor, FailingModule,
    GovFailingModule, IbcFailingModule, StakeKeeper, WasmKeeper,
};
use white_whale_std::fee::PoolFee;
use white_whale_testing::multi_test::stargate_mock::StargateMock;

use crate::state::{CONFIG, REWARD_BUCKETS};
use cw_multi_test::{Contract, ContractWrapper};
use white_whale_std::bonding_manager::{
    BondedResponse, BondingWeightResponse, Config, ExecuteMsg, GlobalIndex, InstantiateMsg,
    QueryMsg, RewardsResponse, UnbondingResponse, WithdrawableResponse,
};
use white_whale_std::bonding_manager::{ClaimableRewardBucketsResponse, RewardBucket};
use white_whale_std::epoch_manager::epoch_manager::{Epoch as EpochV2, EpochConfig};
use white_whale_std::pool_manager::PoolType;

pub fn bonding_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate)
    .with_reply(crate::contract::reply);

    Box::new(contract)
}

fn contract_pool_manager() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        pool_manager::contract::execute,
        pool_manager::contract::instantiate,
        pool_manager::contract::query,
    )
    .with_migrate(pool_manager::contract::migrate)
    .with_reply(pool_manager::contract::reply);

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

type OsmosisTokenFactoryApp = App<
    BankKeeper,
    MockApi,
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
    pub app: OsmosisTokenFactoryApp,
    pub senders: Vec<Addr>,
    pub bonding_manager_addr: Addr,
    pub pool_manager_addr: Addr,
    pub epoch_manager_addr: Addr,
    owned_deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
    env: cosmwasm_std::Env,
}

/// instantiate / execute messages
impl TestingSuite {
    #[track_caller]
    pub(crate) fn default() -> Self {
        let sender = Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3");
        let another_sender = Addr::unchecked("migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40");
        let sender_3 = Addr::unchecked("migaloo1ludaslnu24p5eftw499f7ngsc2jkzqdsrvxt75");

        let bank = BankKeeper::new();
        let initial_balance = vec![
            coin(1_000_000_000_000, "uwhale"),
            coin(1_000_000_000_000, "uusdc"),
            coin(1_000_000_000_000, "ampWHALE"),
            coin(1_000_000_000_000, "bWHALE"),
            coin(1_000_000_000_000, "non_whitelisted_asset"),
        ];

        let balances = vec![
            (sender.clone(), initial_balance.clone()),
            (another_sender.clone(), initial_balance.clone()),
            (sender_3.clone(), initial_balance.clone()),
        ];

        let app = AppBuilder::new()
            // .with_api(MockApiBech32::new("migaloo"))
            .with_wasm(WasmKeeper::default())
            .with_bank(bank)
            .with_stargate(StargateMock {})
            .build(|router, _api, storage| {
                balances.into_iter().for_each(|(account, amount)| {
                    router.bank.init_balance(storage, &account, amount).unwrap()
                });
            });

        Self {
            app: app,
            senders: vec![sender, another_sender, sender_3],
            bonding_manager_addr: Addr::unchecked(""),
            pool_manager_addr: Addr::unchecked(""),
            epoch_manager_addr: Addr::unchecked(""),
            owned_deps: mock_dependencies(),
            env: mock_env(),
        }
    }

    #[track_caller]
    pub(crate) fn fast_forward(&mut self, seconds: u64) -> &mut Self {
        let mut block_info = self.app.block_info();
        block_info.time = block_info.time.plus_nanos(seconds * 1_000_000_000);
        self.app.set_block(block_info);

        self
    }
    #[track_caller]
    pub(crate) fn add_one_day(&mut self) -> &mut Self {
        let mut block_info = self.app.block_info();
        block_info.time = block_info.time.plus_days(1);
        self.app.set_block(block_info);

        self
    }

    #[track_caller]
    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        self.instantiate(
            1u64,
            Decimal::one(),
            vec!["ampWHALE".to_string(), "bWHALE".to_string()],
            &vec![],
        )
    }

    #[track_caller]
    pub(crate) fn instantiate(
        &mut self,
        unbonding_period: u64,
        growth_rate: Decimal,
        bonding_assets: Vec<String>,
        funds: &Vec<Coin>,
    ) -> &mut Self {
        let epoch_manager_id = self.app.store_code(epoch_manager_contract());

        let epoch_manager_addr = self
            .app
            .instantiate_contract(
                epoch_manager_id,
                self.senders[0].clone(),
                &white_whale_std::epoch_manager::epoch_manager::InstantiateMsg {
                    start_epoch: EpochV2 {
                        id: 0,
                        start_time: self.app.block_info().time,
                    },
                    epoch_config: EpochConfig {
                        duration: Uint64::new(86_400_000_000_000u64), // a day
                        genesis_epoch: self.app.block_info().time.nanos().into(),
                    },
                },
                &[],
                "epoch_manager",
                None,
            )
            .unwrap();

        let bonding_manager_addr =
            instantiate_contract(self, unbonding_period, growth_rate, bonding_assets, funds)
                .unwrap();

        let hook_registration_msg =
            white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::AddHook {
                contract_addr: bonding_manager_addr.clone().to_string(),
            };
        let resp = self
            .app
            .execute_contract(
                self.senders[0].clone(),
                epoch_manager_addr.clone(),
                &hook_registration_msg,
                &[],
            )
            .unwrap();

        let msg = white_whale_std::pool_manager::InstantiateMsg {
            bonding_manager_addr: bonding_manager_addr.clone().to_string(),
            incentive_manager_addr: bonding_manager_addr.clone().to_string(),
            pool_creation_fee: Coin {
                amount: Uint128::from(1_000u128),
                denom: "uwhale".to_string(),
            },
        };

        let pool_manager_id = self.app.store_code(contract_pool_manager());

        let creator = self.senders[0].clone();

        let pool_manager_addr = self
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
        let msg = ExecuteMsg::UpdateConfig {
            epoch_manager_addr: Some(epoch_manager_addr.clone().to_string()),
            pool_manager_addr: Some(pool_manager_addr.clone().to_string()),
            growth_rate: None,
            unbonding_period: None,
        };
        self.app
            .execute_contract(
                self.senders[0].clone(),
                bonding_manager_addr.clone(),
                &msg,
                &[],
            )
            .unwrap();

        self.bonding_manager_addr = bonding_manager_addr;
        self.pool_manager_addr = pool_manager_addr;
        self.epoch_manager_addr = epoch_manager_addr;
        self
    }

    #[track_caller]
    pub(crate) fn instantiate_err(
        &mut self,
        unbonding_period: u64,
        growth_rate: Decimal,
        bonding_assets: Vec<String>,
        funds: &Vec<Coin>,
        error: impl Fn(anyhow::Error),
    ) -> &mut Self {
        error(
            instantiate_contract(self, unbonding_period, growth_rate, bonding_assets, funds)
                .unwrap_err(),
        );

        self
    }

    #[track_caller]
    pub(crate) fn bond(
        &mut self,
        sender: Addr,
        _asset: Coin,
        funds: &[Coin],
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Bond {};

        response(
            self.app
                .execute_contract(sender, self.bonding_manager_addr.clone(), &msg, funds),
        );

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
    pub(crate) fn claim(
        &mut self,
        sender: Addr,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Claim {};

        response(
            self.app
                .execute_contract(sender, self.bonding_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn withdraw(
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
    pub(crate) fn create_new_epoch(&mut self) -> &mut Self {
        let new_epoch_msg = white_whale_std::epoch_manager::epoch_manager::ExecuteMsg::CreateEpoch;
        self.app
            .execute_contract(
                self.senders[0].clone(),
                self.epoch_manager_addr.clone(),
                &new_epoch_msg,
                &[],
            )
            .unwrap();

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

    #[track_caller]
    pub(crate) fn update_config(
        &mut self,
        sender: Addr,
        epoch_manager_addr: Option<String>,
        pool_manager_addr: Option<String>,
        unbonding_period: Option<u64>,
        growth_rate: Option<Decimal>,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::UpdateConfig {
            epoch_manager_addr,
            pool_manager_addr,
            unbonding_period,
            growth_rate,
        };

        response(
            self.app
                .execute_contract(sender, self.bonding_manager_addr.clone(), &msg, &[]),
        );

        self
    }
}

fn instantiate_contract(
    suite: &mut TestingSuite,
    unbonding_period: u64,
    growth_rate: Decimal,
    bonding_assets: Vec<String>,
    funds: &Vec<Coin>,
) -> anyhow::Result<Addr, Error> {
    let msg = InstantiateMsg {
        unbonding_period,
        distribution_denom: "uwhale".to_string(),
        growth_rate,
        bonding_assets,
        grace_period: 21u64,
        epoch_manager_addr: "".to_string(),
    };

    let bonding_manager_id = suite.app.store_code(bonding_manager_contract());
    suite.app.instantiate_contract(
        bonding_manager_id,
        suite.senders[0].clone(),
        &msg,
        funds,
        "Bonding Manager".to_string(),
        Some(suite.senders[0].clone().to_string()),
    )
}

/// queries
impl TestingSuite {
    #[track_caller]
    pub(crate) fn query_config(
        &mut self,
        response: impl Fn(StdResult<(&mut Self, Config)>),
    ) -> &mut Self {
        let config: Config = self
            .app
            .wrap()
            .query_wasm_smart(&self.bonding_manager_addr, &QueryMsg::Config {})
            .unwrap();

        response(Ok((self, config)));

        self
    }
    #[track_caller]
    pub(crate) fn query_owner(
        &mut self,
        response: impl Fn(StdResult<(&mut Self, String)>),
    ) -> &mut Self {
        let ownership: cw_ownable::Ownership<String> = self
            .app
            .wrap()
            .query_wasm_smart(&self.bonding_manager_addr, &QueryMsg::Ownership {})
            .unwrap();

        response(Ok((self, ownership.owner.unwrap())));

        self
    }

    #[track_caller]
    pub(crate) fn query_global_index(
        &mut self,
        epoch_id: Option<u64>,
        response: impl Fn(StdResult<(&mut Self, GlobalIndex)>),
    ) -> &mut Self {
        let global_index: GlobalIndex = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.bonding_manager_addr,
                &QueryMsg::GlobalIndex { epoch_id },
            )
            .unwrap();

        response(Ok((self, global_index)));

        self
    }

    #[track_caller]
    pub(crate) fn query_claimable_reward_buckets(
        &mut self,
        address: Option<Addr>,
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
    pub(crate) fn query_unbonding(
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
    pub(crate) fn query_withdrawable(
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
    pub(crate) fn query_rewards(
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

    // Pool Manager methods

    #[track_caller]
    pub(crate) fn provide_liquidity(
        &mut self,
        sender: Addr,
        pool_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::ProvideLiquidity {
            pool_identifier,
            slippage_tolerance: None,
            receiver: None,
            lock_position_identifier: None,
            unlocking_duration: None,
            max_spread: None,
        };

        result(
            self.app
                .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn swap(
        &mut self,
        sender: Addr,
        _offer_asset: Coin,
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
    pub(crate) fn add_swap_routes(
        &mut self,
        sender: Addr,
        swap_routes: Vec<white_whale_std::pool_manager::SwapRoute>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::AddSwapRoutes { swap_routes };

        result(
            self.app
                .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn fill_rewards_lp(
        &mut self,
        sender: Addr,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        result(self.app.execute_contract(
            sender,
            self.bonding_manager_addr.clone(),
            &ExecuteMsg::FillRewards,
            &funds,
        ));

        self
    }

    #[track_caller]
    pub(crate) fn create_pair(
        &mut self,
        sender: Addr,
        asset_denoms: Vec<String>,
        pool_fees: PoolFee,
        pool_type: PoolType,
        pool_identifier: Option<String>,
        pair_creation_fee_funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::pool_manager::ExecuteMsg::CreatePool {
            asset_denoms,
            pool_fees,
            pool_type,
            pool_identifier,
            asset_decimals: vec![6, 6],
        };

        result(self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
            &msg,
            &pair_creation_fee_funds,
        ));

        self
    }
}

use anyhow::Error;
use cosmwasm_std::testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, OwnedDeps, StdResult, Timestamp, Uint64};
use cw_multi_test::{App, AppResponse, Executor};

use crate::state::{EPOCHS, LAST_CLAIMED_EPOCH};
use cw_multi_test::{Contract, ContractWrapper};
use white_whale_std::bonding_manager::Epoch;
use white_whale_std::bonding_manager::{
    BondedResponse, BondingWeightResponse, Config, ExecuteMsg, InstantiateMsg, QueryMsg,
    UnbondingResponse, WithdrawableResponse,
};
use white_whale_std::epoch_manager::epoch_manager::{Epoch as EpochV2, EpochConfig};
use white_whale_std::pool_network::asset::AssetInfo;
use white_whale_testing::integration::contracts::{
    store_epoch_manager_code, store_fee_collector_code, store_fee_distributor_code,
};
use white_whale_testing::integration::integration_mocks::mock_app_with_balance;

pub fn bonding_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_migrate(crate::contract::migrate);

    Box::new(contract)
}
pub struct TestingRobot {
    app: App,
    pub sender: Addr,
    pub another_sender: Addr,
    bonding_manager_addr: Addr,
    owned_deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
    env: cosmwasm_std::Env,
}

/// instantiate / execute messages
impl TestingRobot {
    pub(crate) fn default() -> Self {
        let sender = Addr::unchecked("owner");
        let another_sender = Addr::unchecked("random");

        Self {
            app: mock_app_with_balance(vec![
                (
                    sender.clone(),
                    vec![
                        coin(1_000_000_000, "uwhale"),
                        coin(1_000_000_000, "uusdc"),
                        coin(1_000_000_000, "ampWHALE"),
                        coin(1_000_000_000, "bWHALE"),
                        coin(1_000_000_000, "non_whitelisted_asset"),
                    ],
                ),
                (
                    another_sender.clone(),
                    vec![
                        coin(1_000_000_000, "uwhale"),
                        coin(1_000_000_000, "uusdc"),
                        coin(1_000_000_000, "ampWHALE"),
                        coin(1_000_000_000, "bWHALE"),
                        coin(1_000_000_000, "non_whitelisted_asset"),
                    ],
                ),
            ]),
            sender,
            another_sender,
            bonding_manager_addr: Addr::unchecked(""),
            owned_deps: mock_dependencies(),
            env: mock_env(),
        }
    }

    pub(crate) fn fast_forward(&mut self, seconds: u64) -> &mut Self {
        let mut block_info = self.app.block_info();
        block_info.time = block_info.time.plus_nanos(seconds * 1_000_000_000);
        self.app.set_block(block_info);

        self
    }

    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        self.instantiate(
            Uint64::new(1_000_000_000_000u64),
            Decimal::one(),
            vec!["ampWHALE".to_string(), "bWHALE".to_string()],
            &vec![],
        )
    }

    pub(crate) fn instantiate(
        &mut self,
        unbonding_period: Uint64,
        growth_rate: Decimal,
        bonding_assets: Vec<String>,
        funds: &Vec<Coin>,
    ) -> &mut Self {
        let fee_collector_id = store_fee_collector_code(&mut self.app);
        let fee_distributor_id = store_fee_distributor_code(&mut self.app);

        let epoch_manager_id = store_epoch_manager_code(&mut self.app);

        let _epoch_manager_addr = self
            .app
            .instantiate_contract(
                epoch_manager_id,
                self.sender.clone(),
                &white_whale_std::epoch_manager::epoch_manager::InstantiateMsg {
                    start_epoch: EpochV2 {
                        id: 123,
                        start_time: Timestamp::from_nanos(1678802400_000000000u64),
                    },
                    epoch_config: EpochConfig {
                        duration: Uint64::new(86_400_000_000_000u64), // a day
                        genesis_epoch: Uint64::new(1678802400_000000000u64), // March 14, 2023 2:00:00 PM
                    },
                },
                &[],
                "epoch_manager",
                None,
            )
            .unwrap();

        let fee_collector_address = self
            .app
            .instantiate_contract(
                fee_collector_id,
                self.sender.clone(),
                &white_whale_std::fee_collector::InstantiateMsg {},
                &[],
                "fee_collector",
                None,
            )
            .unwrap();
        println!("fee_collector_address: {}", fee_collector_address);

        let bonding_manager_addr =
            instantiate_contract(self, unbonding_period, growth_rate, bonding_assets, funds)
                .unwrap();
        println!("bonding_manager_addr: {}", bonding_manager_addr);

        let fee_distributor_address = self
            .app
            .instantiate_contract(
                fee_distributor_id,
                self.sender.clone(),
                &white_whale_std::fee_distributor::InstantiateMsg {
                    bonding_contract_addr: bonding_manager_addr.clone().to_string(),
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
        // Now set the fee distributor on the config of the whale lair
        // So that we can check claims before letting them bond/unbond
        let msg = ExecuteMsg::UpdateConfig {
            owner: None,
            unbonding_period: None,
            growth_rate: None,
        };
        self.app
            .execute_contract(self.sender.clone(), bonding_manager_addr.clone(), &msg, &[])
            .unwrap();
        self.bonding_manager_addr = bonding_manager_addr;
        println!("fee_distributor_address: {}", fee_distributor_address);
        self
    }

    pub(crate) fn instantiate_err(
        &mut self,
        unbonding_period: Uint64,
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

    pub(crate) fn update_config(
        &mut self,
        sender: Addr,
        owner: Option<String>,
        unbonding_period: Option<Uint64>,
        growth_rate: Option<Decimal>,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::UpdateConfig {
            owner,
            unbonding_period,
            growth_rate,
        };

        response(
            self.app
                .execute_contract(sender, self.bonding_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    pub(crate) fn add_epochs_to_state(&mut self, epochs: Vec<Epoch>) -> &mut Self {
        for epoch in epochs {
            EPOCHS
                .save(
                    &mut self.owned_deps.storage,
                    &epoch.id.to_be_bytes(),
                    &epoch,
                )
                .unwrap();
        }

        self
    }

    pub(crate) fn add_last_claimed_epoch_to_state(
        &mut self,
        address: Addr,
        epoch_id: Uint64,
    ) -> &mut Self {
        LAST_CLAIMED_EPOCH
            .save(&mut self.owned_deps.storage, &address, &epoch_id)
            .unwrap();
        self
    }
}

fn instantiate_contract(
    robot: &mut TestingRobot,
    unbonding_period: Uint64,
    growth_rate: Decimal,
    bonding_assets: Vec<String>,
    funds: &Vec<Coin>,
) -> anyhow::Result<Addr, Error> {
    let msg = InstantiateMsg {
        unbonding_period,
        growth_rate,
        bonding_assets,
        grace_period: Uint64::new(21),
    };

    let bonding_manager_id = robot.app.store_code(bonding_manager_contract());
    robot.app.instantiate_contract(
        bonding_manager_id,
        robot.sender.clone(),
        &msg,
        funds,
        "White Whale Lair".to_string(),
        Some(robot.sender.clone().to_string()),
    )
}

/// queries
impl TestingRobot {
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

    pub(crate) fn query_weight(
        &mut self,
        address: String,

        response: impl Fn(StdResult<(&mut Self, BondingWeightResponse)>),
    ) -> &mut Self {
        let bonding_weight_response: BondingWeightResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                &self.bonding_manager_addr,
                &QueryMsg::Weight {
                    address,
                    timestamp: Some(self.app.block_info().time),
                    global_index: None,
                },
            )
            .unwrap();

        response(Ok((self, bonding_weight_response)));

        self
    }

    // pub(crate) fn query_claimable_epochs(
    //     &mut self,
    //     address: Option<Addr>,
    //     response: impl Fn(StdResult<(&mut Self, Vec<Epoch>)>),
    // ) -> &mut Self {
    //     let query_res = if let Some(address) = address {
    //         query(
    //             self.owned_deps.as_ref(),
    //             self.env.clone(),
    //             QueryMsg::Claimable {
    //                 address: address.to_string(),
    //             },
    //         )
    //         .unwrap()
    //     } else {
    //         query(
    //             self.owned_deps.as_ref(),
    //             self.env.clone(),
    //             QueryMsg::ClaimableEpochs {},
    //         )
    //         .unwrap()
    //     };

    //     let res: ClaimableEpochsResponse = from_json(query_res).unwrap();

    //     response(Ok((self, res.epochs)));

    //     self
    // }

    pub(crate) fn query_bonded(
        &mut self,
        address: String,
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
        println!("withdrawable_response: {:?}", withdrawable_response);

        response(Ok((self, withdrawable_response)));

        self
    }

    pub(crate) fn query_total_bonded(
        &mut self,
        response: impl Fn(StdResult<(&mut Self, BondedResponse)>),
    ) -> &mut Self {
        let bonded_response: BondedResponse = self
            .app
            .wrap()
            .query_wasm_smart(&self.bonding_manager_addr, &QueryMsg::TotalBonded {})
            .unwrap();

        response(Ok((self, bonded_response)));

        self
    }
}

/// assertions
impl TestingRobot {
    pub(crate) fn assert_config(&mut self, expected: Config) -> &mut Self {
        self.query_config(|res| {
            let config = res.unwrap().1;
            assert_eq!(config, expected);
        });

        self
    }

    pub(crate) fn assert_bonded_response(
        &mut self,
        address: String,
        expected: BondedResponse,
    ) -> &mut Self {
        self.query_bonded(address, |res| {
            let bonded_response = res.unwrap().1;
            assert_eq!(bonded_response, expected);
        })
    }

    pub(crate) fn assert_bonding_weight_response(
        &mut self,
        address: String,
        expected: BondingWeightResponse,
    ) -> &mut Self {
        self.query_weight(address, |res| {
            let bonding_weight_response = res.unwrap().1;
            assert_eq!(bonding_weight_response, expected);
        })
    }

    pub(crate) fn assert_unbonding_response(
        &mut self,
        address: String,
        denom: String,
        expected: UnbondingResponse,
    ) -> &mut Self {
        self.query_unbonding(address, denom, None, None, |res| {
            let unbonding_response = res.unwrap().1;
            assert_eq!(unbonding_response, expected);
        })
    }

    pub(crate) fn assert_withdrawable_response(
        &mut self,
        address: String,
        denom: String,
        expected: WithdrawableResponse,
    ) -> &mut Self {
        self.query_withdrawable(address, denom, |res| {
            let withdrawable_response = res.unwrap().1;
            assert_eq!(withdrawable_response, expected);
        })
    }
}

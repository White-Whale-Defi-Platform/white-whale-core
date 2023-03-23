use cosmwasm_std::{coin, Addr, Coin, Decimal, StdResult, Uint64};
use cw_multi_test::{App, AppResponse, Executor};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::whale_lair::{
    BondedResponse, BondingWeightResponse, Config, ExecuteMsg, InstantiateMsg, QueryMsg,
    UnbondingResponse, WithdrawableResponse,
};
use white_whale_testing::integration::contracts::whale_lair_contract;
use white_whale_testing::integration::integration_mocks::mock_app_with_balance;

pub struct TestingRobot {
    app: App,
    pub sender: Addr,
    pub another_sender: Addr,
    whale_lair_addr: Addr,
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
            whale_lair_addr: Addr::unchecked(""),
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
            vec![
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
            ],
            &vec![],
        )
    }

    pub(crate) fn instantiate(
        &mut self,
        unbonding_period: Uint64,
        growth_rate: Decimal,
        bonding_assets: Vec<AssetInfo>,
        funds: &Vec<Coin>,
    ) -> &mut Self {
        let whale_lair_addr =
            instantiate_contract(self, unbonding_period, growth_rate, bonding_assets, funds)
                .unwrap();
        self.whale_lair_addr = whale_lair_addr;

        self
    }

    pub(crate) fn instantiate_err(
        &mut self,
        unbonding_period: Uint64,
        growth_rate: Decimal,
        bonding_assets: Vec<AssetInfo>,
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
        asset: Asset,
        funds: &[Coin],
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Bond { asset };

        response(
            self.app
                .execute_contract(sender, self.whale_lair_addr.clone(), &msg, funds),
        );

        self
    }

    pub(crate) fn unbond(
        &mut self,
        sender: Addr,
        asset: Asset,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Unbond { asset };

        response(
            self.app
                .execute_contract(sender, self.whale_lair_addr.clone(), &msg, &[]),
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
                .execute_contract(sender, self.whale_lair_addr.clone(), &msg, &[]),
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
                .execute_contract(sender, self.whale_lair_addr.clone(), &msg, &[]),
        );

        self
    }
}

fn instantiate_contract(
    robot: &mut TestingRobot,
    unbonding_period: Uint64,
    growth_rate: Decimal,
    bonding_assets: Vec<AssetInfo>,
    funds: &Vec<Coin>,
) -> anyhow::Result<Addr> {
    let msg = InstantiateMsg {
        unbonding_period,
        growth_rate,
        bonding_assets,
    };

    let whale_lair_id = robot.app.store_code(whale_lair_contract());
    robot.app.instantiate_contract(
        whale_lair_id,
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
            .query_wasm_smart(&self.whale_lair_addr, &QueryMsg::Config {})
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
            .query_wasm_smart(&self.whale_lair_addr, &QueryMsg::Weight { address })
            .unwrap();

        response(Ok((self, bonding_weight_response)));

        self
    }

    pub(crate) fn query_bonded(
        &mut self,
        address: String,
        response: impl Fn(StdResult<(&mut Self, BondedResponse)>),
    ) -> &mut Self {
        let bonded_response: BondedResponse = self
            .app
            .wrap()
            .query_wasm_smart(&self.whale_lair_addr, &QueryMsg::Bonded { address })
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
                &self.whale_lair_addr,
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
                &self.whale_lair_addr,
                &QueryMsg::Withdrawable { address, denom },
            )
            .unwrap();

        response(Ok((self, withdrawable_response)));

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

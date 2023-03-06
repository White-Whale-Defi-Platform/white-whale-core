use cosmwasm_std::testing::{mock_dependencies, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, Addr, Coin, DepsMut, Empty, Env, MessageInfo, OwnedDeps, Response,
    StdResult, Uint128,
};
use cw_multi_test::{App, AppResponse, Executor};
use serde::de::StdError;

use white_whale::whale_lair::{
    Asset, AssetInfo, BondedResponse, BondingWeightResponse, Config, ExecuteMsg, InstantiateMsg,
    QueryMsg,
};
use white_whale_testing::integration::contracts::whale_lair_contract;
use white_whale_testing::integration::integration_mocks::mock_app_with_balance;

use crate::contract::{instantiate, query};
use crate::ContractError;

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

    pub(crate) fn fast_forward(&mut self, blocks: u64) -> &mut Self {
        self.app
            .update_block(|b| b.height = b.height.checked_add(blocks).unwrap());

        self
    }

    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        self.instantiate(
            1_000u64,
            1u8,
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
        unbonding_period: u64,
        growth_rate: u8,
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
        unbonding_period: u64,
        growth_rate: u8,
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

    pub(crate) fn update_config(
        &mut self,
        sender: Addr,
        owner: Option<String>,
        unbonding_period: Option<u64>,
        growth_rate: Option<u8>,
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::UpdateConfig {
            owner,
            unbonding_period,
            growth_rate,
        };

        response(
            self.app
                .execute_contract(sender, self.whale_lair_addr.clone(), &msg, &vec![]),
        );

        self
    }
}

fn instantiate_contract(
    robot: &mut TestingRobot,
    unbonding_period: u64,
    growth_rate: u8,
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

    pub(crate) fn query_bonded_err(
        &mut self,
        address: String,
        response: impl Fn(StdResult<(&mut Self, StdResult<BondedResponse>)>),
    ) -> &mut Self {
        let res = self
            .app
            .wrap()
            .query_wasm_smart(&self.whale_lair_addr, &QueryMsg::Bonded { address });

        response(Ok((self, res)));

        self
    }
}

/// assertions
impl TestingRobot {
    pub(crate) fn assert_error(
        &mut self,
        found: anyhow::Error,
        expected: ContractError,
    ) -> &mut Self {
        self
    }

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
}

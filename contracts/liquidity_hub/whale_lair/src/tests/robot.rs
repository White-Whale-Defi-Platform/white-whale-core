use cosmwasm_std::{Addr, Coin, coin, coins, DepsMut, Empty, Env, from_binary, MessageInfo, OwnedDeps, Response, StdResult, Uint128};
use cosmwasm_std::testing::{mock_dependencies, MockApi, MockQuerier, MockStorage};
use cw_multi_test::{App, Executor};

use white_whale::whale_lair::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};
use white_whale_testing::integration::contracts::whale_lair_contract;
use white_whale_testing::integration::integration_mocks::mock_app_with_balance;

use crate::contract::{instantiate, query};
use crate::ContractError;

pub struct TestingRobot {
    app: App,
    sender: Addr,
    whale_lair_addr: Addr,
}

/// instantiate / execute
impl TestingRobot {
    pub(crate) fn default() -> Self {
        let sender = Addr::unchecked("owner");

        Self {
            app: mock_app_with_balance(vec![(sender.clone(), coins(1_000_000_000, "uwhale"))]),
            sender,
            whale_lair_addr: Addr::unchecked(""),
        }
    }

    pub(crate) fn instantiate(
        &mut self,
        unstaking_period: u64,
        growth_rate: u8,
        staking_denom: String,
        funds: &Vec<Coin>,
    ) -> &mut Self {
        let whale_lair_addr =
            instantiate_contract(self, unstaking_period, growth_rate, staking_denom, funds)
                .unwrap();
        self.whale_lair_addr = whale_lair_addr;

        self
    }

    pub(crate) fn instantiate_err(
        &mut self,
        unstaking_period: u64,
        growth_rate: u8,
        staking_denom: String,
        funds: &Vec<Coin>,
    ) -> &mut Self {
        instantiate_contract(self, unstaking_period, growth_rate, staking_denom, funds)
            .unwrap_err();
        self
    }

    pub(crate) fn stake(
        &mut self,
        amount: Uint128,
    ) -> &mut Self {
        let msg = ExecuteMsg::Stake { amount };

        self.app.execute_contract(
            self.sender.clone(),
            self.whale_lair_addr.clone(),
            &msg,
            &[coin(amount.u128(), "uwhale")])
            .unwrap();

        self
    }
}

fn instantiate_contract(
    robot: &mut TestingRobot,
    unstaking_period: u64,
    growth_rate: u8,
    staking_denom: String,
    funds: &Vec<Coin>,
) -> anyhow::Result<Addr> {
    let msg = InstantiateMsg {
        unstaking_period,
        growth_rate,
        staking_denom,
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
}

/// assertions points
impl TestingRobot {
    pub(crate) fn assert_config(&mut self, expected: Config) -> &mut Self {
        self.query_config(|res| {
            let config = res.unwrap().1;
            assert_eq!(config, expected);
        });

        self
    }
}

use cosmwasm_std::testing::{mock_dependencies, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    coin, coins, from_binary, Addr, Coin, DepsMut, Empty, Env, MessageInfo, OwnedDeps, Response,
    StdResult, Uint128,
};
use cw_multi_test::{App, AppResponse, Executor};

use white_whale::whale_lair::{Config, ExecuteMsg, InstantiateMsg, QueryMsg};
use white_whale_testing::integration::contracts::whale_lair_contract;
use white_whale_testing::integration::integration_mocks::mock_app_with_balance;

use crate::contract::{instantiate, query};
use crate::ContractError;

pub struct TestingRobot {
    app: App,
    pub sender: Addr,
    whale_lair_addr: Addr,
}

/// instantiate / execute messages
impl TestingRobot {
    pub(crate) fn default() -> Self {
        let sender = Addr::unchecked("owner");

        Self {
            app: mock_app_with_balance(vec![(
                sender.clone(),
                vec![coin(1_000_000_000, "uwhale"), coin(1_000_000_000, "uusdc")],
            )]),
            sender,
            whale_lair_addr: Addr::unchecked(""),
        }
    }

    pub(crate) fn instantiate(
        &mut self,
        unbonding_period: u64,
        growth_rate: u8,
        bonding_denom: String,
        funds: &Vec<Coin>,
    ) -> &mut Self {
        let whale_lair_addr =
            instantiate_contract(self, unbonding_period, growth_rate, bonding_denom, funds)
                .unwrap();
        self.whale_lair_addr = whale_lair_addr;

        self
    }

    pub(crate) fn instantiate_err(
        &mut self,
        unbonding_period: u64,
        growth_rate: u8,
        bonding_denom: String,
        funds: &Vec<Coin>,
    ) -> &mut Self {
        instantiate_contract(self, unbonding_period, growth_rate, bonding_denom, funds)
            .unwrap_err();
        self
    }

    pub(crate) fn bond(
        &mut self,
        funds: &[Coin],
        response: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ExecuteMsg::Bond {
            asset: funds[0].amount,
        };

        response(self.app.execute_contract(
            self.sender.clone(),
            self.whale_lair_addr.clone(),
            &msg,
            funds,
        ));

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
    bonding_denom: String,
    funds: &Vec<Coin>,
) -> anyhow::Result<Addr> {
    let msg = InstantiateMsg {
        unbonding_period,
        growth_rate,
        bonding_assets: bonding_denom,
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

/// assertions
impl TestingRobot {
    pub(crate) fn assert_config(&mut self, expected: Config) -> &mut Self {
        self.query_config(|res| {
            let config = res.unwrap().1;
            assert_eq!(config, expected);
        });

        self
    }
}

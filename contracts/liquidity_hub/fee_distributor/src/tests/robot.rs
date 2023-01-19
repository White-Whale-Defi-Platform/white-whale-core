use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{Empty, Env, MessageInfo, OwnedDeps, Response};
use serde::{Deserialize, Serialize};

use crate::contract::instantiate;
use crate::msg::InstantiateMsg;
use crate::state::{Epoch, EPOCHS};
use crate::ContractError;

pub struct TestingRobot {
    deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
    env: Env,
    info: MessageInfo,
}

impl TestingRobot {
    pub(crate) fn new(
        deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        env: Env,
        info: MessageInfo,
    ) -> Self {
        Self { deps, env, info }
    }

    pub(crate) fn instantiate(
        mut self,
        staking_contract_addr: String,
        fee_collector_addr: String,
        grace_period: u128,
    ) -> Result<Response, ContractError> {
        let msg = InstantiateMsg {
            staking_contract_addr,
            fee_collector_addr,
            grace_period,
        };

        let deps = self.deps.as_mut();
        let env = self.env;
        let info = self.info;

        instantiate(deps, env, info, msg)
    }

    pub(crate) fn add_epochs_to_state(&self, epochs: Vec<Epoch>) -> () {
        let mut storage = &self.deps.storage;

        storage = storage.clone();

        // for epoch in epochs {
        //     EPOCHS
        //         .save(
        //             storage,
        //             &epoch.id.to_be_bytes(),
        //             &epoch,
        //         )
        //         .unwrap();
        // }
    }

    pub(crate) fn test(self) -> () {}
}

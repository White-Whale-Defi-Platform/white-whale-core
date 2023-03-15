use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{from_binary, Empty, Env, MessageInfo, OwnedDeps, Response, StdResult, Uint64};

use white_whale::fee_distributor::{
    Config, Epoch, EpochConfig, ExecuteMsg, InstantiateMsg, QueryMsg,
};
use white_whale::pool_network::asset::Asset;

use crate::contract::{execute, instantiate, query};
use crate::state::{get_expiring_epoch, EPOCHS};
use crate::ContractError;

pub struct TestingRobot {
    owned_deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
    env: Env,
}

impl TestingRobot {
    pub(crate) fn new(
        owned_deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        env: Env,
    ) -> Self {
        Self { owned_deps, env }
    }

    pub(crate) fn instantiate(
        &mut self,
        info: MessageInfo,
        staking_contract_addr: String,
        fee_collector_addr: String,
        grace_period: Uint64,
        epoch_config: EpochConfig,
    ) -> &mut Self {
        let msg = InstantiateMsg {
            bonding_contract_addr: staking_contract_addr,
            fee_collector_addr,
            grace_period,
            epoch_config,
        };

        instantiate(self.owned_deps.as_mut(), self.env.clone(), info, msg).unwrap();

        self
    }

    pub(crate) fn instantiate_err(
        &mut self,
        info: MessageInfo,
        staking_contract_addr: String,
        fee_collector_addr: String,
        grace_period: Uint64,
        epoch_config: EpochConfig,
    ) -> &mut Self {
        let msg = InstantiateMsg {
            bonding_contract_addr: staking_contract_addr,
            fee_collector_addr,
            grace_period,
            epoch_config,
        };

        instantiate(self.owned_deps.as_mut(), self.env.clone(), info, msg).unwrap_err();

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

    pub(crate) fn create_new_epoch(
        &mut self,
        info: MessageInfo,
        response: impl Fn(Result<Response, ContractError>),
    ) -> &mut Self {
        //create new epoch with ExecuteMsg::NewEpoch
        let msg = ExecuteMsg::NewEpoch {};

        response(execute(
            self.owned_deps.as_mut(),
            self.env.clone(),
            info,
            msg,
        ));

        self
    }

    pub(crate) fn update_config(
        &mut self,
        info: MessageInfo,
        config: Config,
        response: impl Fn(Result<Response, ContractError>),
    ) -> &mut Self {
        let msg = ExecuteMsg::UpdateConfig {
            owner: Some(config.owner.to_string()),
            staking_contract_addr: Some(config.staking_contract_addr.to_string()),
            fee_collector_addr: Some(config.fee_collector_addr.to_string()),
            grace_period: Some(config.grace_period),
        };

        response(execute(
            self.owned_deps.as_mut(),
            self.env.clone(),
            info,
            msg,
        ));

        self
    }

    /// Queries

    pub(crate) fn query_current_epoch(&mut self, response: impl Fn(StdResult<Epoch>)) -> &mut Self {
        let query_res = query(
            self.owned_deps.as_ref(),
            self.env.clone(),
            QueryMsg::CurrentEpoch {},
        )
        .unwrap();
        let current_epoch: Epoch = from_binary(&query_res).unwrap();

        response(Ok(current_epoch));

        self
    }

    pub(crate) fn query_epoch(
        &mut self,
        id: u128,
        response: impl Fn(StdResult<(&mut Self, Epoch)>),
    ) -> &mut Self {
        let query_res = query(
            self.owned_deps.as_ref(),
            self.env.clone(),
            QueryMsg::Epoch { id },
        )
        .unwrap();
        let epoch: Epoch = from_binary(&query_res).unwrap();

        response(Ok((self, epoch)));

        self
    }

    pub(crate) fn query_claimable_epochs(
        &mut self,
        response: impl Fn(StdResult<(&mut Self, Vec<Epoch>)>),
    ) -> &mut Self {
        let query_res = query(
            self.owned_deps.as_ref(),
            self.env.clone(),
            QueryMsg::ClaimableEpochs {},
        )
        .unwrap();
        let epochs: Vec<Epoch> = from_binary(&query_res).unwrap();

        response(Ok((self, epochs)));

        self
    }

    pub(crate) fn query_config(
        &mut self,
        response: impl Fn(StdResult<(&mut Self, Config)>),
    ) -> &mut Self {
        let query_res = query(
            self.owned_deps.as_ref(),
            self.env.clone(),
            QueryMsg::Config {},
        )
        .unwrap();
        let config: Config = from_binary(&query_res).unwrap();

        response(Ok((self, config)));

        self
    }

    /// Assertions

    pub(crate) fn assert_current_epoch(&mut self, expected: &Epoch) -> &mut Self {
        self.query_current_epoch(|epoch| {
            assert_eq!(&epoch.unwrap(), expected);
        });

        self
    }

    pub(crate) fn assert_expiring_epoch(&mut self, expected: Option<&Epoch>) -> &mut Self {
        let expiring_epoch = get_expiring_epoch(self.owned_deps.as_ref()).unwrap();
        assert_eq!(expiring_epoch.as_ref(), expected);
        self
    }

    pub(crate) fn asset_config(&mut self, expected: Config) -> &mut Self {
        self.query_config(|config| {
            assert_eq!(config.unwrap().1, expected);
        });

        self
    }
}

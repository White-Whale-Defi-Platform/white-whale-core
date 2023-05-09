use cosmwasm_std::testing::{mock_info, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    from_binary, Addr, Empty, Env, MessageInfo, OwnedDeps, Response, StdResult, Uint64,
};

use white_whale::fee_distributor::{
    ClaimableEpochsResponse, Config, Epoch, EpochConfig, EpochResponse, ExecuteMsg, InstantiateMsg,
    LastClaimedEpochResponse, QueryMsg,
};
use white_whale::pool_network::asset::AssetInfo;

use crate::contract::{execute, instantiate, query};
use crate::state::{get_expiring_epoch, EPOCHS, LAST_CLAIMED_EPOCH};
use crate::ContractError;

pub struct TestingRobot {
    owned_deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
    pub env: Env,
}

impl TestingRobot {
    pub(crate) fn new(
        owned_deps: OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
        env: Env,
    ) -> Self {
        Self { owned_deps, env }
    }

    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        let msg = InstantiateMsg {
            bonding_contract_addr: "bonding_contract_addr".to_string(),
            fee_collector_addr: "fee_collector_addr".to_string(),
            grace_period: Uint64::new(2),
            epoch_config: EpochConfig {
                duration: Uint64::new(86_400_000_000_000u64), // a day
                genesis_epoch: Uint64::new(1678802400_000000000u64), // March 14, 2023 2:00:00 PM
            },
            distribution_asset: AssetInfo::NativeToken {
                denom: "uwhale".to_string(),
            },
        };

        instantiate(
            self.owned_deps.as_mut(),
            self.env.clone(),
            mock_info("owner", &[]),
            msg,
        )
        .unwrap();

        self
    }

    pub(crate) fn instantiate(
        &mut self,
        info: MessageInfo,
        bonding_contract_addr: String,
        fee_collector_addr: String,
        grace_period: Uint64,
        epoch_config: EpochConfig,
        distribution_asset: AssetInfo,
    ) -> &mut Self {
        let msg = InstantiateMsg {
            bonding_contract_addr,
            fee_collector_addr,
            grace_period,
            epoch_config,
            distribution_asset,
        };

        instantiate(self.owned_deps.as_mut(), self.env.clone(), info, msg).unwrap();

        self
    }

    pub(crate) fn instantiate_err(
        &mut self,
        info: MessageInfo,
        bonding_contract_addr: String,
        fee_collector_addr: String,
        grace_period: Uint64,
        epoch_config: EpochConfig,
        distribution_asset: AssetInfo,
    ) -> &mut Self {
        let msg = InstantiateMsg {
            bonding_contract_addr,
            fee_collector_addr,
            grace_period,
            epoch_config,
            distribution_asset,
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

    pub(crate) fn update_config(
        &mut self,
        info: MessageInfo,
        config: Config,
        response: impl Fn(Result<Response, ContractError>),
    ) -> &mut Self {
        let msg = ExecuteMsg::UpdateConfig {
            owner: Some(config.owner.to_string()),
            bonding_contract_addr: Some(config.bonding_contract_addr.to_string()),
            fee_collector_addr: Some(config.fee_collector_addr.to_string()),
            grace_period: Some(config.grace_period),
            distribution_asset: Some(config.distribution_asset),
            epoch_config: Some(config.epoch_config),
        };

        response(execute(
            self.owned_deps.as_mut(),
            self.env.clone(),
            info,
            msg,
        ));

        self
    }

    pub(crate) fn create_new_epoch(
        &mut self,
        info: MessageInfo,
        response: impl Fn(Result<Response, ContractError>),
    ) -> &mut Self {
        let msg = ExecuteMsg::NewEpoch {};

        response(execute(
            self.owned_deps.as_mut(),
            self.env.clone(),
            info,
            msg,
        ));

        self
    }

    pub(crate) fn set_last_claimed_epoch(
        &mut self,
        info: MessageInfo,
        address: Addr,
        epoch_id: Uint64,
        response: impl Fn(Result<Response, ContractError>),
    ) -> &mut Self {
        let msg = ExecuteMsg::SetLastClaimedEpoch {
            address: address.to_string(),
            epoch_id,
        };

        response(execute(
            self.owned_deps.as_mut(),
            self.env.clone(),
            info,
            msg,
        ));

        self
    }
}

/// Queries
impl TestingRobot {
    pub(crate) fn query_current_epoch(&mut self, response: impl Fn(StdResult<Epoch>)) -> &mut Self {
        let query_res = query(
            self.owned_deps.as_ref(),
            self.env.clone(),
            QueryMsg::CurrentEpoch {},
        )
        .unwrap();
        let res: EpochResponse = from_binary(&query_res).unwrap();

        response(Ok(res.epoch));

        self
    }

    pub(crate) fn query_epoch(
        &mut self,
        id: Uint64,
        response: impl Fn(StdResult<(&mut Self, Epoch)>),
    ) -> &mut Self {
        let query_res = query(
            self.owned_deps.as_ref(),
            self.env.clone(),
            QueryMsg::Epoch { id },
        )
        .unwrap();
        let res: EpochResponse = from_binary(&query_res).unwrap();

        response(Ok((self, res.epoch)));

        self
    }

    pub(crate) fn query_claimable_epochs(
        &mut self,
        address: Option<Addr>,
        response: impl Fn(StdResult<(&mut Self, Vec<Epoch>)>),
    ) -> &mut Self {
        let query_res = if let Some(address) = address {
            query(
                self.owned_deps.as_ref(),
                self.env.clone(),
                QueryMsg::Claimable {
                    address: address.to_string(),
                },
            )
            .unwrap()
        } else {
            query(
                self.owned_deps.as_ref(),
                self.env.clone(),
                QueryMsg::ClaimableEpochs {},
            )
            .unwrap()
        };

        let res: ClaimableEpochsResponse = from_binary(&query_res).unwrap();

        response(Ok((self, res.epochs)));

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

    pub(crate) fn query_last_claimed_epoch(
        &mut self,
        address: Addr,
        response: impl Fn(StdResult<(&mut Self, LastClaimedEpochResponse)>),
    ) -> &mut Self {
        let query_res = query(
            self.owned_deps.as_ref(),
            self.env.clone(),
            QueryMsg::LastClaimedEpoch {
                address: address.to_string(),
            },
        )
        .unwrap();
        let res: LastClaimedEpochResponse = from_binary(&query_res).unwrap();

        response(Ok((self, res)));

        self
    }
}

/// Assertions
impl TestingRobot {
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

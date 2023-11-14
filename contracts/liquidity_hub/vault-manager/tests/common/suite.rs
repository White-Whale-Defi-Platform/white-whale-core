use cosmwasm_std::{Addr, Coin, Decimal, StdResult, Timestamp, Uint128, Uint64};
use cw20::{Cw20Coin, MinterResponse};
use cw_multi_test::{App, AppBuilder, AppResponse, BankKeeper, Executor};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::vault_manager::{InstantiateMsg, LpTokenType};

use crate::common::suite_contracts::{
    cw20_token_contract, vault_manager_contract, whale_lair_contract,
};

pub struct TestingSuite {
    app: App,
    pub senders: [Addr; 3],
    pub whale_lair_addr: Addr,
    pub vault_manager_addr: Addr,
    pub cw20_tokens: Vec<Addr>,
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

    pub(crate) fn get_time(&mut self) -> Timestamp {
        self.app.block_info().time
    }

    pub(crate) fn increase_allowance(
        &mut self,
        sender: Addr,
        cw20contract: Addr,
        allowance: Uint128,
        spender: Addr,
    ) -> &mut Self {
        let msg = cw20_base::msg::ExecuteMsg::IncreaseAllowance {
            spender: spender.to_string(),
            amount: allowance,
            expires: None,
        };

        self.app
            .execute_contract(sender, cw20contract, &msg, &vec![])
            .unwrap();

        self
    }
}

/// Instantiate
impl TestingSuite {
    pub(crate) fn default() -> Self {
        let sender_1 = Addr::unchecked("alice");
        let sender_2 = Addr::unchecked("bob");
        let sender_3 = Addr::unchecked("carol");

        Self {
            app: App::default(),
            senders: [sender_1, sender_2, sender_3],
            whale_lair_addr: Addr::unchecked(""),
            vault_manager_addr: Addr::unchecked(""),
            cw20_tokens: vec![],
        }
    }

    pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
        let sender_1 = Addr::unchecked("alice");
        let sender_2 = Addr::unchecked("bob");
        let sender_3 = Addr::unchecked("carol");

        let bank = BankKeeper::new();

        let balances = vec![
            (sender_1.clone(), initial_balance.clone()),
            (sender_2.clone(), initial_balance.clone()),
            (sender_3.clone(), initial_balance.clone()),
        ];

        let app = AppBuilder::new()
            .with_bank(bank)
            .build(|router, _api, storage| {
                balances.into_iter().for_each(|(account, amount)| {
                    router.bank.init_balance(storage, &account, amount).unwrap()
                });
            });

        Self {
            app,
            senders: [sender_1, sender_2, sender_3],
            whale_lair_addr: Addr::unchecked(""),
            vault_manager_addr: Addr::unchecked(""),
            cw20_tokens: vec![],
        }
    }

    #[track_caller]
    pub(crate) fn instantiate_default(&mut self) -> &mut Self {
        self.create_whale_lair();
        self.create_cw20_token();

        // 17 May 2023 17:00:00 UTC
        let timestamp = Timestamp::from_seconds(1684342800u64);
        self.set_time(timestamp);

        self.instantiate(
            self.whale_lair_addr.to_string(),
            LpTokenType::TokenFactory,
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
        )
    }

    #[track_caller]
    pub(crate) fn instantiate_with_cw20_lp_token(&mut self) -> &mut Self {
        self.create_whale_lair();
        let cw20_code_id = self.create_cw20_token();

        // 17 May 2023 17:00:00 UTC
        let timestamp = Timestamp::from_seconds(1684342800u64);
        self.set_time(timestamp);

        self.instantiate(
            self.whale_lair_addr.to_string(),
            LpTokenType::Cw20(cw20_code_id),
            Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(1_000u128),
            },
        )
    }

    fn create_whale_lair(&mut self) {
        let whale_lair_id = self.app.store_code(whale_lair_contract());

        // create whale lair
        let msg = white_whale::whale_lair::InstantiateMsg {
            unbonding_period: Uint64::new(86400u64),
            growth_rate: Decimal::one(),
            bonding_assets: vec![
                AssetInfo::NativeToken {
                    denom: "bWHALE".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "ampWHALE".to_string(),
                },
            ],
        };

        let creator = self.creator().clone();

        self.whale_lair_addr = self
            .app
            .instantiate_contract(
                whale_lair_id,
                creator.clone(),
                &msg,
                &[],
                "White Whale Lair".to_string(),
                Some(creator.to_string()),
            )
            .unwrap();
    }
    pub fn create_cw20_token(&mut self) -> u64 {
        let msg = white_whale::pool_network::token::InstantiateMsg {
            name: "mocktoken".to_string(),
            symbol: "MOCK".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: self.senders[0].to_string(),
                    amount: Uint128::new(1_000_000_000_000u128),
                },
                Cw20Coin {
                    address: self.senders[1].to_string(),
                    amount: Uint128::new(1_000_000_000_000u128),
                },
                Cw20Coin {
                    address: self.senders[2].to_string(),
                    amount: Uint128::new(1_000_000_000_000u128),
                },
            ],
            mint: Some(MinterResponse {
                minter: self.senders[0].to_string(),
                cap: None,
            }),
        };

        let cw20_token_id = self.app.store_code(cw20_token_contract());

        let creator = self.creator().clone();

        self.cw20_tokens.append(&mut vec![self
            .app
            .instantiate_contract(
                cw20_token_id,
                creator.clone(),
                &msg,
                &[],
                "mock cw20 token",
                Some(creator.into_string()),
            )
            .unwrap()]);
        cw20_token_id
    }

    #[track_caller]
    pub(crate) fn instantiate(
        &mut self,
        whale_lair_addr: String,
        lp_token_type: LpTokenType,
        vault_creation_fee: Asset,
    ) -> &mut Self {
        let msg = InstantiateMsg {
            owner: self.creator().to_string(),
            lp_token_type,
            whale_lair_addr,
            vault_creation_fee,
        };

        let vault_manager_id = self.app.store_code(vault_manager_contract());

        let creator = self.creator().clone();

        self.vault_manager_addr = self
            .app
            .instantiate_contract(
                vault_manager_id,
                creator.clone(),
                &msg,
                &[],
                "mock vault manager",
                Some(creator.into_string()),
            )
            .unwrap();
        self
    }

    #[track_caller]
    pub(crate) fn instantiate_err(
        &mut self,
        lp_token_type: LpTokenType,
        vault_creation_fee: Asset,
        error: impl Fn(anyhow::Error),
    ) -> &mut Self {
        let creator = self.creator().clone();

        let msg = InstantiateMsg {
            owner: creator.clone().to_string(),
            lp_token_type,
            whale_lair_addr: self.whale_lair_addr.to_string(),
            vault_creation_fee,
        };

        let vault_manager_id = self.app.store_code(vault_manager_contract());

        let err = self
            .app
            .instantiate_contract(
                vault_manager_id,
                creator.clone(),
                &msg,
                &[],
                "mock vault manager",
                Some(creator.into_string()),
            )
            .unwrap_err();

        error(err);
        self
    }
}

/// execute messages
impl TestingSuite {
    pub(crate) fn update_ownership(
        &mut self,
        sender: Addr,
        action: cw_ownable::Action,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::vault_manager::ExecuteMsg::UpdateOwnership(action);

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    pub(crate) fn create_vault(
        &mut self,
        sender: Addr,
        asset_info: AssetInfo,
        fees: white_whale::vault_manager::VaultFee,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::vault_manager::ExecuteMsg::CreateVault { asset_info, fees };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
        );

        self
    }
}

/// queries
impl TestingSuite {
    pub(crate) fn query_ownership(
        &mut self,
        result: impl Fn(StdResult<cw_ownable::Ownership<String>>),
    ) -> &mut Self {
        let ownership_response: StdResult<cw_ownable::Ownership<String>> =
            self.app.wrap().query_wasm_smart(
                &self.vault_manager_addr,
                &white_whale::vault_manager::QueryMsg::Ownership {},
            );

        result(ownership_response);

        self
    }
}

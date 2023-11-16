use cosmwasm_std::{
    coin, to_json_binary, Addr, Coin, Decimal, StdResult, Timestamp, Uint128, Uint64,
};
use cw20::{BalanceResponse, Cw20Coin, MinterResponse};
use cw_multi_test::{App, AppBuilder, AppResponse, BankKeeper, Executor, WasmKeeper};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::vault_manager::{
    Cw20HookMsg, Cw20ReceiveMsg, FilterVaultBy, InstantiateMsg, LpTokenType, VaultsResponse,
};

use crate::common::suite_contracts::{
    cw20_token_contract, vault_manager_contract, whale_lair_contract,
};
use crate::common::test_addresses::MockAddressGenerator;
use crate::common::test_api::MockApiBech32;

pub struct TestingSuite {
    app: App<BankKeeper, MockApiBech32>,
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
    pub(crate) fn default_with_balances(initial_balance: Vec<Coin>) -> Self {
        let sender_1 = Addr::unchecked("migaloo1h3s5np57a8cxaca3rdjlgu8jzmr2d2zz55s5y3");
        let sender_2 = Addr::unchecked("migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40");
        let sender_3 = Addr::unchecked("migaloo1ludaslnu24p5eftw499f7ngsc2jkzqdsrvxt75");

        let bank = BankKeeper::new();

        let balances = vec![
            (sender_1.clone(), initial_balance.clone()),
            (sender_2.clone(), initial_balance.clone()),
            (sender_3.clone(), initial_balance.clone()),
        ];

        let app = AppBuilder::new()
            .with_api(MockApiBech32::new("migaloo"))
            .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
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

    #[track_caller]
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
    #[track_caller]
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

    #[track_caller]
    pub(crate) fn create_vault(
        &mut self,
        sender: Addr,
        asset_info: AssetInfo,
        vault_identifier: Option<String>,
        fees: white_whale::vault_manager::VaultFee,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::vault_manager::ExecuteMsg::CreateVault {
            asset_info,
            fees,
            vault_identifier,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn deposit(
        &mut self,
        sender: Addr,
        asset: Asset,
        vault_identifier: String,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::vault_manager::ExecuteMsg::Deposit {
            asset,
            vault_identifier,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &funds),
        );

        self
    }

    #[track_caller]
    pub(crate) fn withdraw(
        &mut self,
        sender: Addr,
        asset: Asset,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        match asset.info {
            AssetInfo::Token { contract_addr } => {
                let msg = cw20::Cw20ExecuteMsg::Send {
                    contract: self.vault_manager_addr.to_string(),
                    amount: asset.amount,
                    msg: to_json_binary(&Cw20HookMsg::Withdraw).unwrap(),
                };

                result(self.app.execute_contract(
                    sender,
                    Addr::unchecked(contract_addr),
                    &msg,
                    &funds,
                ));
            }
            AssetInfo::NativeToken { .. } => {
                unimplemented!()
            }
        }

        self
    }
    #[track_caller]
    pub(crate) fn update_config(
        &mut self,
        sender: Addr,
        whale_lair_addr: Option<String>,
        vault_creation_fee: Option<Asset>,
        cw20_lp_code_id: Option<u64>,
        flash_loan_enabled: Option<bool>,
        deposit_enabled: Option<bool>,
        withdraw_enabled: Option<bool>,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale::vault_manager::ExecuteMsg::UpdateConfig {
            whale_lair_addr,
            vault_creation_fee,
            cw20_lp_code_id,
            flash_loan_enabled,
            deposit_enabled,
            withdraw_enabled,
        };

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

    #[track_caller]
    pub(crate) fn query_vault(
        &mut self,
        filter_by: FilterVaultBy,
        result: impl Fn(StdResult<VaultsResponse>),
    ) -> &mut Self {
        let vaults_response: StdResult<VaultsResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale::vault_manager::QueryMsg::Vault { filter_by },
        );

        result(vaults_response);

        self
    }

    #[track_caller]
    pub(crate) fn query_vaults(
        &mut self,
        start_after: Option<Vec<u8>>,
        limit: Option<u32>,
        result: impl Fn(StdResult<VaultsResponse>),
    ) -> &mut Self {
        let vaults_response: StdResult<VaultsResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale::vault_manager::QueryMsg::Vaults { start_after, limit },
        );

        result(vaults_response);

        self
    }
    #[track_caller]
    pub(crate) fn query_balance(
        &mut self,
        asset_info: AssetInfo,
        address: Addr,
        result: impl Fn(Uint128),
    ) -> &mut Self {
        let balance: Uint128 = match asset_info {
            AssetInfo::Token { contract_addr } => {
                let balance_response: StdResult<BalanceResponse> =
                    self.app.wrap().query_wasm_smart(
                        &contract_addr,
                        &cw20_base::msg::QueryMsg::Balance {
                            address: address.to_string(),
                        },
                    );

                if balance_response.is_err() {
                    Uint128::zero()
                } else {
                    balance_response.unwrap().balance
                }
            }
            AssetInfo::NativeToken { denom } => {
                let balance_response = self.app.wrap().query_balance(address, denom.clone());

                balance_response.unwrap_or(coin(0, denom)).amount
            }
        };

        result(balance);

        self
    }
}

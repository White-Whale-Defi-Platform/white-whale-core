use cosmwasm_std::{
    coin, to_json_binary, Addr, Coin, CosmosMsg, Decimal, StdResult, Timestamp, Uint128, Uint64,
};
use cw20::{BalanceResponse, Cw20Coin, MinterResponse};
use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{App, AppBuilder, AppResponse, BankKeeper, Executor, Wasm, WasmKeeper};

use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairType};
use white_whale_std::pool_network::pair::ExecuteMsg::{ProvideLiquidity, Swap};
use white_whale_std::pool_network::pair::{PoolFee, SimulationResponse};
use white_whale_std::vault_manager::{
    Config, Cw20HookMsg, FilterVaultBy, InstantiateMsg, LpTokenType, PaybackAssetResponse,
    ShareResponse, VaultsResponse,
};

use crate::common::suite_contracts::{
    cw20_token_contract, fee_collector_contract, pair_contract, vault_manager_contract,
    whale_lair_contract,
};

pub struct TestingSuite {
    app: App<BankKeeper, MockApiBech32>,
    pub senders: [Addr; 3],
    pub whale_lair_addr: Addr,
    pub vault_manager_addr: Addr,
    pub cw20_tokens: Vec<Addr>,
    pub pools: Vec<Addr>,
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

    #[track_caller]
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

    #[track_caller]
    pub(crate) fn create_pool(
        &mut self,
        asset_infos: [AssetInfo; 2],
        asset_decimals: [u8; 2],
        pool_fees: PoolFee,
        pair_type: PairType,
        token_factory_lp: bool,
    ) -> &mut Self {
        let pair_id = self.app.store_code(pair_contract());
        let token_code_id = self.app.store_code(cw20_token_contract());
        let fee_collector = self.create_fee_collector();

        // create whale lair
        let msg = white_whale_std::pool_network::pair::InstantiateMsg {
            asset_infos,
            token_code_id,
            asset_decimals,
            pool_fees,
            fee_collector_addr: fee_collector.to_string(),
            pair_type,
            token_factory_lp,
        };

        let creator = self.creator().clone();

        self.pools.append(&mut vec![self
            .app
            .instantiate_contract(
                pair_id,
                creator.clone(),
                &msg,
                &[],
                "pool",
                Some(creator.into_string()),
            )
            .unwrap()]);

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
            pools: vec![],
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
        let msg = white_whale_std::whale_lair::InstantiateMsg {
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

    fn create_fee_collector(&mut self) -> Addr {
        let fee_collector_contract = self.app.store_code(fee_collector_contract());

        // create whale lair
        let msg = white_whale_std::fee_collector::InstantiateMsg {};

        let creator = self.creator().clone();

        self.app
            .instantiate_contract(
                fee_collector_contract,
                creator.clone(),
                &msg,
                &[],
                "White Whale Lair".to_string(),
                Some(creator.to_string()),
            )
            .unwrap()
    }

    #[track_caller]
    pub fn create_cw20_token(&mut self) -> u64 {
        let msg = white_whale_std::pool_network::token::InstantiateMsg {
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
        let msg = white_whale_std::vault_manager::ExecuteMsg::UpdateOwnership(action);

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
        fees: white_whale_std::vault_manager::VaultFee,
        funds: Vec<Coin>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::vault_manager::ExecuteMsg::CreateVault {
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
        let msg = white_whale_std::vault_manager::ExecuteMsg::Deposit {
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
                let msg = white_whale_std::vault_manager::ExecuteMsg::Withdraw {};

                let vault_manager = self.vault_manager_addr.clone();

                result(
                    self.app
                        .execute_contract(sender, vault_manager, &msg, &funds),
                );
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
        let msg = white_whale_std::vault_manager::ExecuteMsg::UpdateConfig {
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

    #[track_caller]
    pub(crate) fn flashloan(
        &mut self,
        sender: Addr,
        asset: Asset,
        vault_identifier: String,
        payload: Vec<CosmosMsg>,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = white_whale_std::vault_manager::ExecuteMsg::FlashLoan {
            asset,
            vault_identifier,
            payload,
        };

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &[]),
        );

        self
    }

    #[track_caller]
    pub(crate) fn callback(
        &mut self,
        sender: Addr,
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        // the values here don't matter as this is the test it would give an error, as only the
        // contract itself can make callbacks
        let msg = white_whale_std::vault_manager::ExecuteMsg::Callback(
            white_whale_std::vault_manager::CallbackMsg::AfterFlashloan {
                old_asset_balance: Uint128::zero(),
                loan_asset: Asset {
                    info: AssetInfo::NativeToken {
                        denom: "".to_string(),
                    },
                    amount: Default::default(),
                },
                sender: sender.clone(),
                vault_identifier: "".to_string(),
            },
        );

        result(
            self.app
                .execute_contract(sender, self.vault_manager_addr.clone(), &msg, &[]),
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
                &white_whale_std::vault_manager::QueryMsg::Ownership {},
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
            &white_whale_std::vault_manager::QueryMsg::Vault { filter_by },
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
            &white_whale_std::vault_manager::QueryMsg::Vaults { start_after, limit },
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
    #[track_caller]
    pub(crate) fn query_config(&mut self, result: impl Fn(StdResult<Config>)) -> &mut Self {
        let response: StdResult<Config> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::vault_manager::QueryMsg::Config {},
        );

        result(response);

        self
    }
    #[track_caller]
    pub(crate) fn query_share(
        &mut self,
        lp_share: Asset,
        result: impl Fn(StdResult<ShareResponse>),
    ) -> &mut Self {
        let response: StdResult<ShareResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::vault_manager::QueryMsg::Share { lp_share },
        );

        result(response);

        self
    }

    #[track_caller]
    pub(crate) fn query_payback(
        &mut self,
        asset: Asset,
        vault_identifier: String,
        result: impl Fn(StdResult<PaybackAssetResponse>),
    ) -> &mut Self {
        let response: StdResult<PaybackAssetResponse> = self.app.wrap().query_wasm_smart(
            &self.vault_manager_addr,
            &white_whale_std::vault_manager::QueryMsg::PaybackAmount {
                asset,
                vault_identifier,
            },
        );

        result(response);

        self
    }
}

// pools interactions
impl TestingSuite {
    #[track_caller]
    pub(crate) fn provide_liquidity(
        &mut self,
        sender: Addr,
        assets: [Asset; 2],
        pool: Addr,
        funds: &[Coin],
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = ProvideLiquidity {
            assets,
            slippage_tolerance: None,
            receiver: None,
        };

        result(self.app.execute_contract(sender, pool, &msg, funds));

        self
    }
    #[track_caller]
    pub(crate) fn swap(
        &mut self,
        sender: Addr,
        offer_asset: Asset,
        pool: Addr,
        funds: &[Coin],
        result: impl Fn(Result<AppResponse, anyhow::Error>),
    ) -> &mut Self {
        let msg = Swap {
            offer_asset,
            belief_price: None,
            max_spread: None,
            to: None,
        };

        result(self.app.execute_contract(sender, pool, &msg, funds));

        self
    }

    #[track_caller]
    pub(crate) fn simulate_swap(
        &mut self,
        offer_asset: Asset,
        pool: Addr,
        result: impl Fn(StdResult<SimulationResponse>),
    ) -> &mut Self {
        let response: StdResult<SimulationResponse> = self.app.wrap().query_wasm_smart(
            pool,
            &white_whale_std::pool_network::pair::QueryMsg::Simulation { offer_asset },
        );

        result(response);

        self
    }
}

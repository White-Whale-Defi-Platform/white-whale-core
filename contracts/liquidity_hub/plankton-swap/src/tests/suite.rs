use std::collections::HashMap;

use crate::{
    contract::Cw20HookMsg,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::NPairInfo,
};
use anyhow::{Ok, Result as AnyResult};
use cosmwasm_std::{to_binary, Addr, Coin, Decimal, Empty, Uint128, Timestamp, Uint64};
use cw20::{Cw20Coin, MinterResponse};
use cw_multi_test::{
    App, AppBuilder, AppResponse, BankKeeper, Contract, ContractWrapper, Executor, Router,
    WasmKeeper,
};
use white_whale::{
    fee::Fee,
    pool_network::{
        asset::{Asset, AssetInfo, PairType},
        pair::PoolFee,
    }, vault_manager::LpTokenType,
};

use super::MockAPIBech32::{MockAddressGenerator, MockApiBech32};
fn contract_pool_manager() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );

    Box::new(contract)
}

fn cw20_token_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );

    Box::new(contract)
}

/// Creates the whale lair contract
pub fn whale_lair_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        whale_lair::contract::execute,
        whale_lair::contract::instantiate,
        whale_lair::contract::query,
    )
    .with_migrate(whale_lair::contract::migrate);

    Box::new(contract)
}


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
    pub(crate) fn instantiate(
        &mut self,
        whale_lair_addr: String,
        lp_token_type: LpTokenType,
        vault_creation_fee: Asset,
    ) -> &mut Self {
        let cw20_token_id = self.app.store_code(cw20_token_contract());
        let msg = InstantiateMsg {
            fee_collector_addr: whale_lair_addr,
            token_code_id: cw20_token_id,
            pair_code_id: cw20_token_id,
            owner: self.creator().to_string(),
            pool_creation_fee: Asset {
                amount: Uint128::from(100u128),
                info: AssetInfo::NativeToken {
                    denom: "uusd".to_string(),
                },
            },
        };

        let vault_manager_id = self.app.store_code(contract_pool_manager());

        let creator = self.creator().clone();

        self.vault_manager_addr = self
            .app
            .instantiate_contract(
                vault_manager_id,
                creator.clone(),
                &msg,
                &[],
                "mock pool manager",
                Some(creator.into_string()),
            )
            .unwrap();
        self
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

}

#[derive(Debug)]
pub struct SuiteBuilder {
    pub cw20_balances: Vec<Cw20Coin>,
    pub native_balances: Vec<(Addr, Coin)>,
}

impl SuiteBuilder {
    pub fn new() -> Self {
        Self {
            cw20_balances: vec![],
            native_balances: vec![],
        }
    }

    pub fn with_native_balances(mut self, denom: &str, balances: Vec<(&str, u128)>) -> Self {
        self.native_balances
            .extend(balances.into_iter().map(|(addr, amount)| {
                (
                    Addr::unchecked(addr),
                    Coin {
                        denom: denom.to_owned(),
                        amount: amount.into(),
                    },
                )
            }));
        self
    }

    pub fn with_cw20_balances(mut self, balances: Vec<(&str, u128)>) -> Self {
        let initial_balances = balances
            .into_iter()
            .map(|(address, amount)| Cw20Coin {
                address: address.to_owned(),
                amount: amount.into(),
            })
            .collect::<Vec<Cw20Coin>>();
        self.cw20_balances = initial_balances;
        self
    }

    #[track_caller]
    pub fn build(self) -> Suite {
        // Default app
        let mut app: App = AppBuilder::new().build(|_, _, _| {});

        // Instantiate2 version
        // prepare wasm module with custom address generator
        // let wasm_keeper: WasmKeeper<Empty, Empty> =
        //     WasmKeeper::new().with_address_generator(MockAddressGenerator);

        // prepare application with custom api
        let mut app = AppBuilder::new()
            .with_wasm::<WasmKeeper<Empty, Empty>>(
                WasmKeeper::new().with_address_generator(MockAddressGenerator),
            )
            .with_api(MockApiBech32::new("migaloo"))
            .build(|_, _, _| {});
        // provide initial native balances
        app.init_modules(|router, _, storage| {
            // group by address
            let mut balances = HashMap::<Addr, Vec<Coin>>::new();
            for (addr, coin) in self.native_balances {
                let addr_balance = balances.entry(addr).or_default();
                addr_balance.push(coin);
            }

            for (addr, coins) in balances {
                router
                    .bank
                    .init_balance(storage, &addr, coins)
                    .expect("init balance");
            }
        });

        let admin = Addr::unchecked("admin");
        let test_account = app.api().addr_make("addr0000");
        let pool_manager_id = contract_pool_manager(&mut app);
        let token_contract_code_id = store_token_code(&mut app);

        let pool_manager_addr = app
            .instantiate_contract(
                pool_manager_id,
                admin.clone(),
                &InstantiateMsg {
                    fee_collector_addr: app.api().addr_make("fee_collector_addr").to_string(),
                    token_code_id: token_contract_code_id,
                    pair_code_id: token_contract_code_id,
                    owner: app.api().addr_make("owner").to_string(),
                    pool_creation_fee: Asset {
                        amount: Uint128::from(100u128),
                        info: AssetInfo::NativeToken {
                            denom: "uusd".to_string(),
                        },
                    },
                },
                &[],
                "pool_manager",
                None,
            )
            .unwrap();

        Suite {
            app,
            pool_manager_addr,
            test_account: test_account,
        }
    }
}

pub struct Suite {
    pub app: App<BankKeeper, MockApiBech32>,
    pub pool_manager_addr: Addr,
    pub test_account: Addr,
}

impl Suite {
    pub fn create_constant_product_pool(
        &mut self,
        sender: Addr,
        asset_infos_array: Vec<AssetInfo>,
        pool_creation_fee: Uint128,
    ) -> AnyResult<AppResponse> {
        // Convert the Vec<AssetInfo> into a [AssetInfo; 2]
        let mut asset_infos_array = [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::NativeToken {
                denom: "fable".to_string(),
            },
        ];
        let msg = ExecuteMsg::CreatePair {
            asset_infos: asset_infos_array.to_vec(),
            pool_fees: PoolFee {
                protocol_fee: Fee {
                    share: Decimal::zero(),
                },
                swap_fee: Fee {
                    share: Decimal::zero(),
                },
                burn_fee: Fee {
                    share: Decimal::zero(),
                },
            },
            pair_type: PairType::ConstantProduct,
            token_factory_lp: false,
            pair_identifier: None,
        };

        let res = self.app.execute_contract(
            sender,
            self.pool_manager_addr.clone(),
            &msg,
            &[Coin {
                denom: "uusd".to_string(),
                amount: pool_creation_fee,
            }],
        )?;
        Ok(res)
    }

    pub(crate) fn add_liquidity(
        &mut self,
        sender: Addr,
        vec: Vec<Asset>,
        funds: &Vec<Coin>,
        pair_identifier: String,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::ProvideLiquidity {
            assets: vec,
            slippage_tolerance: None,
            receiver: None,
            pair_identifier,
        };

        let res = self
            .app
            .execute_contract(sender, self.pool_manager_addr.clone(), &msg, funds)?;
        Ok(res)
    }

    pub(crate) fn withdraw_liquidity(
        &mut self,
        sender: Addr,
        vec: Vec<Asset>,
        funds: &Vec<Coin>,
        pair_identifier: String,
    ) -> AnyResult<AppResponse> {
        // Get the token from config
        let pair_resp: NPairInfo = self.app.wrap().query_wasm_smart(
            self.pool_manager_addr.clone(),
            &crate::msg::QueryMsg::Pair {
                pair_identifier: pair_identifier.clone(),
            },
        )?;

        let msg = ExecuteMsg::WithdrawLiquidity {
            assets: vec.clone(),
            pair_identifier: pair_identifier.clone(),
        };

        let res = self.app.execute_contract(
            sender.clone(),
            self.pool_manager_addr.clone(),
            &msg,
            funds,
        )?;
        Ok(res)
    }

    pub(crate) fn withdraw_liquidity_cw20(
        &mut self,
        sender: Addr,
        vec: Vec<Asset>,
        pair_identifier: String,
        cw20_amount: Uint128,
    ) -> AnyResult<AppResponse> {
        // Get the token from config
        let pair_resp: NPairInfo = self.app.wrap().query_wasm_smart(
            self.pool_manager_addr.clone(),
            &crate::msg::QueryMsg::Pair {
                pair_identifier: pair_identifier.clone(),
            },
        )?;

        // Send the cw20 amount with a message
        let msg = ExecuteMsg::WithdrawLiquidity {
            assets: vec.clone(),
            pair_identifier: pair_identifier.clone(),
        };

        let contract_addr = match pair_resp.liquidity_token {
            AssetInfo::Token { contract_addr } => contract_addr,
            _ => {
                panic!("Liquidity token is not a cw20 token")
            }
        };

        let res = self.app.execute_contract(
            sender.clone(),
            Addr::unchecked(contract_addr),
            &cw20::Cw20ExecuteMsg::Send {
                contract: self.pool_manager_addr.to_string(),
                amount: cw20_amount,
                msg: to_binary(&Cw20HookMsg::WithdrawLiquidity {
                    pair_identifier: "0".to_string(),
                })
                .unwrap(),
            },
            &[],
        )?;

        Ok(res)
    }

    pub(crate) fn add_native_token_decimals(
        &mut self,
        sender: Addr,
        denom: String,
        decimals: u8,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::AddNativeTokenDecimals {
            denom: denom.clone(),
            decimals,
        };
        let res = self
            .app
            .execute_contract(
                sender,
                self.pool_manager_addr.clone(),
                &msg,
                &[Coin {
                    denom: denom.to_string(),
                    amount: Uint128::from(1u128),
                }],
            )
            .unwrap();
        Ok(res)
    }
}

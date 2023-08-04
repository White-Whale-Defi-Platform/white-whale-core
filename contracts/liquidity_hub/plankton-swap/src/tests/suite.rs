use std::collections::HashMap;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use white_whale::{
    fee::Fee,
    pool_network::{
        asset::{AssetInfo, PairType, Asset},
        pair::PoolFee,
    },
};

fn contract_pool_manager(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    ));

    app.store_code(contract)
}

fn store_token_code(app: &mut App) -> u64 {
    let contract = Box::new(ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    ));

    app.store_code(contract)
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
        let mut app: App = App::default();
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

        let pool_manager_id = contract_pool_manager(&mut app);
        let token_contract_code_id = store_token_code(&mut app);

        let pool_manager_addr = app
            .instantiate_contract(
                pool_manager_id,
                admin.clone(),
                &InstantiateMsg {
                    fee_collector_addr: "fee_collector_addr".to_string(),
                    token_code_id: token_contract_code_id,
                    pair_code_id: token_contract_code_id,
                    owner: "owner".to_string(),
                    pool_creation_fee: vec![],
                },
                &[],
                "pool_manager",
                None,
            )
            .unwrap();

        Suite {
            app,
            pool_manager_addr,
        }
    }
}

pub struct Suite {
    pub app: App,
    pool_manager_addr: Addr,
}

impl Suite {
    pub fn create_constant_product_pool(
        &mut self,
        sender: Addr,
        asset_infos: Vec<AssetInfo>,
    ) -> AnyResult<AppResponse> {
        // Convert the Vec<AssetInfo> into a [AssetInfo; 2]
        let mut asset_infos_array = [
            AssetInfo::NativeToken {
                denom: "uusd".to_string(),
            },
            AssetInfo::Token {
                contract_addr: "addr0000".to_string(),
            },
        ];
        let msg = ExecuteMsg::CreatePair {
            asset_infos: crate::state::NAssets::TWO(asset_infos_array),
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
        };

        let res = self
            .app
            .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &[])?;
        Ok(res)
    }

    pub(crate) fn add_liquidity(&mut self, sender: Addr, vec: Vec<Asset>) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::ProvideLiquidity {
            assets: vec,
            slippage_tolerance: None,
            receiver: None,
        };

        let res = self
            .app
            .execute_contract(sender, self.pool_manager_addr.clone(), &msg, &[])?;
        Ok(res)
    }
}

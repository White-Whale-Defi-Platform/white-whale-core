use std::collections::HashMap;

use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockApi, MockQuerier, MockStorage},
    to_binary, Coin, ContractResult, Empty, OwnedDeps, Querier, QuerierResult, QueryRequest,
    SystemError, SystemResult, Uint128, WasmQuery,
};
use cw20::{AllowanceResponse, BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

pub fn mock_dependencies_lp(
    native_balances: &[(&str, &[Coin])],
    token_balances: &[(String, &[(String, Uint128)])],
    token_allowances: Vec<(String, String, Uint128)>,
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let querier = WasmMockQuerier::new(
        MockQuerier::new(native_balances),
        token_balances,
        token_allowances,
    );

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier,
        custom_query_type: std::marker::PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    token_querier: TokenQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
    // key is in the form of (owner, spender)
    allowances: HashMap<(String, String), Uint128>,
}

impl TokenQuerier {
    pub fn new(
        balances: &[(String, &[(String, Uint128)])],
        allowances: Vec<(String, String, Uint128)>,
    ) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
            allowances: allowances_to_map(allowances),
        }
    }
}

fn balances_to_map(
    balances: &[(String, &[(String, Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), *balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

fn allowances_to_map(
    allowances: Vec<(String, String, Uint128)>,
) -> HashMap<(String, String), Uint128> {
    let mut allowances_map = HashMap::new();

    for (owner, spender, allowance) in allowances.into_iter() {
        allowances_map.insert((owner, spender), allowance);
    }

    allowances_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                });
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        println!("Request hit the mock querier: {:?}", request);
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                if contract_addr == "lp_token" {
                    match from_binary(msg).unwrap() {
                        Cw20QueryMsg::TokenInfo {} => {
                            return SystemResult::Ok(ContractResult::Ok(
                                to_binary(&TokenInfoResponse {
                                    decimals: 6,
                                    name: "lp_token".to_string(),
                                    symbol: "uLP".to_string(),
                                    total_supply: self
                                        .token_querier
                                        .balances
                                        .iter()
                                        .filter_map(|account| account.1.get("lp_token"))
                                        .sum(),
                                })
                                .unwrap(),
                            ));
                        }
                        Cw20QueryMsg::Balance { address } => {
                            return SystemResult::Ok(ContractResult::Ok(
                                to_binary(&BalanceResponse {
                                    balance: *self
                                        .token_querier
                                        .balances
                                        .get(&address)
                                        .expect("Address did not have CW20 balance")
                                        .get("lp_token")
                                        .unwrap_or(&Uint128::new(0)),
                                })
                                .unwrap(),
                            ));
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                } else if contract_addr == "vault_token" {
                    match from_binary(msg).unwrap() {
                        Cw20QueryMsg::Balance { address } => {
                            return SystemResult::Ok(ContractResult::Ok(
                                to_binary(&BalanceResponse {
                                    balance: *self
                                        .token_querier
                                        .balances
                                        .get(&address)
                                        .expect("Address did not have CW20 balance")
                                        .get("vault_token")
                                        .unwrap_or(&Uint128::new(0)),
                                })
                                .unwrap(),
                            ));
                        }
                        Cw20QueryMsg::Allowance { owner, spender } => {
                            return SystemResult::Ok(ContractResult::Ok(
                                to_binary(&AllowanceResponse {
                                    allowance: *self
                                        .token_querier
                                        .allowances
                                        .get(&(owner, spender))
                                        .unwrap_or(&Uint128::new(0)),
                                    expires: cw20::Expiration::Never {},
                                })
                                .unwrap(),
                            ))
                        }
                        _ => panic!("DO NOT ENTER HERE"),
                    }
                }

                panic!("DO NOT ENTER HERE")
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(
        base: MockQuerier<Empty>,
        balances: &[(String, &[(String, Uint128)])],
        allowances: Vec<(String, String, Uint128)>,
    ) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::new(balances, allowances),
        }
    }
}

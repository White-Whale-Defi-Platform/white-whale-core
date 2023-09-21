use std::collections::HashMap;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::panic;

use cosmwasm_std::testing::{MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractInfoResponse, ContractResult, Empty,
    OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery, CodeInfoResponse, HexBinary, Addr,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

use crate::pool_network::asset::{Asset, AssetInfo, PairInfo, PairType, TrioInfo};
use crate::pool_network::factory::{NativeTokenDecimalsResponse, QueryMsg as FactoryQueryMsg};
use crate::pool_network::pair::{PoolResponse as PairPoolResponse, QueryMsg as PairQueryMsg};
use crate::pool_network::pair::{ReverseSimulationResponse, SimulationResponse};
use crate::pool_network::trio;
use crate::pool_network::trio::{PoolResponse as TrioPoolResponse, QueryMsg as TrioQueryMsg};
use crate::pool_network::temp_mock_api::MockSimpleApi as MockApi;
/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub fn mock_dependencies_trio(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockTrioQuerier> {
    let custom_querier: WasmMockTrioQuerier =
        WasmMockTrioQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier,
    token_querier: TokenQuerier,
    pool_factory_querier: PoolFactoryQuerier,
}

pub struct WasmMockTrioQuerier {
    base: MockQuerier,
    token_querier: TokenQuerier,
    pool_factory_querier: PoolFactoryQuerier,
}

#[derive(Clone, Default)]
pub struct TokenQuerier {
    // this lets us iterate over all pairs that match the first string
    balances: HashMap<String, HashMap<String, Uint128>>,
}

impl TokenQuerier {
    pub fn new(balances: &[(&String, &[(&String, &Uint128)])]) -> Self {
        TokenQuerier {
            balances: balances_to_map(balances),
        }
    }
}

pub fn balances_to_map(
    balances: &[(&String, &[(&String, &Uint128)])],
) -> HashMap<String, HashMap<String, Uint128>> {
    let mut balances_map: HashMap<String, HashMap<String, Uint128>> = HashMap::new();
    for (contract_addr, balances) in balances.iter() {
        let mut contract_balances_map: HashMap<String, Uint128> = HashMap::new();
        for (addr, balance) in balances.iter() {
            contract_balances_map.insert(addr.to_string(), **balance);
        }

        balances_map.insert(contract_addr.to_string(), contract_balances_map);
    }
    balances_map
}

#[derive(Clone, Default)]
pub struct PoolFactoryQuerier {
    pairs: HashMap<String, PairInfo>,
    native_token_decimals: HashMap<String, u8>,
}

impl PoolFactoryQuerier {
    pub fn new(pairs: &[(&String, &PairInfo)], native_token_decimals: &[(String, u8)]) -> Self {
        PoolFactoryQuerier {
            pairs: pairs_to_map(pairs),
            native_token_decimals: native_token_decimals_to_map(native_token_decimals),
        }
    }
}

pub fn pairs_to_map(pairs: &[(&String, &PairInfo)]) -> HashMap<String, PairInfo> {
    let mut pairs_map: HashMap<String, PairInfo> = HashMap::new();
    for (key, pair) in pairs.iter() {
        let mut sort_key: Vec<char> = key.chars().collect();
        sort_key.sort_by(|a, b| b.cmp(a));
        pairs_map.insert(String::from_iter(sort_key.iter()), (**pair).clone());
    }
    pairs_map
}

pub fn native_token_decimals_to_map(native_token_decimals: &[(String, u8)]) -> HashMap<String, u8> {
    let mut native_token_decimals_map: HashMap<String, u8> = HashMap::new();

    for (denom, decimals) in native_token_decimals.iter() {
        native_token_decimals_map.insert(denom.to_string(), *decimals);
    }
    native_token_decimals_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {e}"),
                    request: bin_request.into(),
                });
            }
        };
        self.handle_query(&request)
    }
}

impl Querier for WasmMockTrioQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {e}"),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(msg) {
                    Ok(FactoryQueryMsg::Pair { asset_infos }) => {
                        let key = [asset_infos[0].to_string(), asset_infos[1].to_string()].join("");
                        let mut sort_key: Vec<char> = key.chars().collect();
                        sort_key.sort_by(|a, b| b.cmp(a));
                        match self
                            .pool_factory_querier
                            .pairs
                            .get(&String::from_iter(sort_key.iter()))
                        {
                            Some(v) => SystemResult::Ok(ContractResult::Ok(to_binary(v).unwrap())),
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "No pair info exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    Ok(FactoryQueryMsg::NativeTokenDecimals { denom }) => {
                        match self.pool_factory_querier.native_token_decimals.get(&denom) {
                            Some(decimals) => SystemResult::Ok(ContractResult::Ok(
                                to_binary(&NativeTokenDecimalsResponse {
                                    decimals: *decimals,
                                })
                                .unwrap(),
                            )),
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "No decimal info exist".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    _ => match from_binary(msg) {
                        Ok(PairQueryMsg::Pool {}) => {
                            SystemResult::Ok(ContractResult::from(to_binary(&PairPoolResponse {
                                assets: vec![
                                    Asset {
                                        info: AssetInfo::NativeToken {
                                            denom: "uluna".to_string(),
                                        },
                                        amount: Uint128::new(1_000_000_000u128),
                                    },
                                    Asset {
                                        info: AssetInfo::NativeToken {
                                            denom: "ujuno".to_string(),
                                        },
                                        amount: Uint128::new(1_000_000_000u128),
                                    },
                                ],
                                total_share: Uint128::new(2_000_000_000u128),
                            })))
                        }
                        Ok(PairQueryMsg::Pair {}) => {
                            SystemResult::Ok(ContractResult::from(to_binary(&PairInfo {
                                asset_infos: [
                                    AssetInfo::NativeToken {
                                        denom: "uluna".to_string(),
                                    },
                                    AssetInfo::NativeToken {
                                        denom: "uluna".to_string(),
                                    },
                                ],
                                asset_decimals: [6u8, 6u8],
                                contract_addr: "pair0000".to_string(),
                                liquidity_token: AssetInfo::Token {
                                    contract_addr: "liquidity0000".to_string(),
                                },
                                pair_type: PairType::ConstantProduct,
                            })))
                        }
                        Ok(PairQueryMsg::Simulation { offer_asset }) => {
                            SystemResult::Ok(ContractResult::from(to_binary(&SimulationResponse {
                                return_amount: offer_asset.amount,
                                swap_fee_amount: Uint128::zero(),
                                spread_amount: Uint128::zero(),
                                protocol_fee_amount: Uint128::zero(),
                                burn_fee_amount: Uint128::zero(),
                            })))
                        }
                        Ok(PairQueryMsg::ReverseSimulation { ask_asset }) => SystemResult::Ok(
                            ContractResult::from(to_binary(&ReverseSimulationResponse {
                                offer_amount: ask_asset.amount,
                                swap_fee_amount: Uint128::zero(),
                                spread_amount: Uint128::zero(),
                                protocol_fee_amount: Uint128::zero(),
                                burn_fee_amount: Uint128::zero(),
                            })),
                        ),
                        _ => match from_binary(msg).unwrap() {
                            Cw20QueryMsg::TokenInfo {} => {
                                let balances: &HashMap<String, Uint128> =
                                match self.token_querier.balances.get(contract_addr) {
                                    Some(balances) => balances,
                                    None => {
                                        return SystemResult::Err(SystemError::InvalidRequest {
                                            error: format!(
                                                "No balance info exists for the contract {contract_addr}"
                                            ),
                                            request: msg.as_slice().into(),
                                        })
                                    }
                                };

                                let mut total_supply = Uint128::zero();

                                for balance in balances {
                                    total_supply += *balance.1;
                                }

                                SystemResult::Ok(ContractResult::Ok(
                                    to_binary(&TokenInfoResponse {
                                        name: "mAAPL".to_string(),
                                        symbol: "mAAPL".to_string(),
                                        decimals: 8,
                                        total_supply,
                                    })
                                    .unwrap(),
                                ))
                            }
                            Cw20QueryMsg::Balance { address } => {
                                let balances: &HashMap<String, Uint128> =
                                match self.token_querier.balances.get(contract_addr) {
                                    Some(balances) => balances,
                                    None => {
                                        return SystemResult::Err(SystemError::InvalidRequest {
                                            error: format!(
                                                "No balance info exists for the contract {contract_addr}"
                                            ),
                                            request: msg.as_slice().into(),
                                        })
                                    }
                                };

                                let balance = match balances.get(&address) {
                                    Some(v) => *v,
                                    None => {
                                        return SystemResult::Ok(ContractResult::Ok(
                                            to_binary(&Cw20BalanceResponse {
                                                balance: Uint128::zero(),
                                            })
                                            .unwrap(),
                                        ));
                                    }
                                };

                                SystemResult::Ok(ContractResult::Ok(
                                    to_binary(&Cw20BalanceResponse { balance }).unwrap(),
                                ))
                            }

                            _ => panic!("DO NOT ENTER HERE"),
                        },
                    },
                }
            }
            QueryRequest::Wasm(WasmQuery::ContractInfo { .. }) => {
                let mut contract_info_response = ContractInfoResponse::default();
                contract_info_response.code_id = 0u64;
                contract_info_response.creator = "creator".to_string();
                contract_info_response.admin = Some("creator".to_string());

                SystemResult::Ok(ContractResult::Ok(
                    to_binary(&contract_info_response).unwrap(),
                ))
            },
            QueryRequest::Wasm(WasmQuery::CodeInfo { code_id }) => {
                let mut default = CodeInfoResponse::default();

                match code_id {
                    11 => {
                        default.code_id = 67;
                        default.creator = Addr::unchecked("creator").to_string();
                        default.checksum =HexBinary::from_hex(&sha256::digest(format!("code_checksum_{}", code_id)))
                        .unwrap();
                        SystemResult::Ok(to_binary(&default).into())
                    }
                    _ => {
                        return SystemResult::Err(SystemError::Unknown {});
                    }
                }
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockTrioQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(msg) {
                    Ok(FactoryQueryMsg::Trio { asset_infos }) => {
                        let key = [
                            asset_infos[0].to_string(),
                            asset_infos[1].to_string(),
                            asset_infos[2].to_string(),
                        ]
                        .join("");
                        let mut sort_key: Vec<char> = key.chars().collect();
                        sort_key.sort_by(|a, b| b.cmp(a));
                        match self
                            .pool_factory_querier
                            .pairs
                            .get(&String::from_iter(sort_key.iter()))
                        {
                            Some(v) => SystemResult::Ok(ContractResult::Ok(to_binary(v).unwrap())),
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "No trio info exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    Ok(FactoryQueryMsg::NativeTokenDecimals { denom }) => {
                        match self.pool_factory_querier.native_token_decimals.get(&denom) {
                            Some(decimals) => SystemResult::Ok(ContractResult::Ok(
                                to_binary(&NativeTokenDecimalsResponse {
                                    decimals: *decimals,
                                })
                                .unwrap(),
                            )),
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "No decimal info exist".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    _ => match from_binary(msg) {
                        Ok(TrioQueryMsg::Pool {}) => {
                            SystemResult::Ok(ContractResult::from(to_binary(&TrioPoolResponse {
                                assets: vec![
                                    Asset {
                                        info: AssetInfo::NativeToken {
                                            denom: "uluna".to_string(),
                                        },
                                        amount: Uint128::new(1_000_000_000u128),
                                    },
                                    Asset {
                                        info: AssetInfo::NativeToken {
                                            denom: "ujuno".to_string(),
                                        },
                                        amount: Uint128::new(1_000_000_000u128),
                                    },
                                    Asset {
                                        info: AssetInfo::NativeToken {
                                            denom: "uatom".to_string(),
                                        },
                                        amount: Uint128::new(1_000_000_000u128),
                                    },
                                ],
                                total_share: Uint128::new(3_000_000_000u128),
                            })))
                        }
                        Ok(TrioQueryMsg::Trio {}) => {
                            SystemResult::Ok(ContractResult::from(to_binary(&TrioInfo {
                                asset_infos: [
                                    AssetInfo::NativeToken {
                                        denom: "uluna".to_string(),
                                    },
                                    AssetInfo::NativeToken {
                                        denom: "uluna".to_string(),
                                    },
                                    AssetInfo::NativeToken {
                                        denom: "uatom".to_string(),
                                    },
                                ],
                                asset_decimals: [6u8, 6u8, 10u8],
                                contract_addr: "trio0000".to_string(),
                                liquidity_token: AssetInfo::Token {
                                    contract_addr: "liquidity0000".to_string(),
                                },
                            })))
                        }
                        Ok(TrioQueryMsg::Simulation { offer_asset, .. }) => SystemResult::Ok(
                            ContractResult::from(to_binary(&trio::SimulationResponse {
                                return_amount: offer_asset.amount,
                                swap_fee_amount: Uint128::zero(),
                                spread_amount: Uint128::zero(),
                                protocol_fee_amount: Uint128::zero(),
                                burn_fee_amount: Uint128::zero(),
                            })),
                        ),
                        Ok(TrioQueryMsg::ReverseSimulation { ask_asset, .. }) => SystemResult::Ok(
                            ContractResult::from(to_binary(&trio::ReverseSimulationResponse {
                                offer_amount: ask_asset.amount,
                                swap_fee_amount: Uint128::zero(),
                                spread_amount: Uint128::zero(),
                                protocol_fee_amount: Uint128::zero(),
                                burn_fee_amount: Uint128::zero(),
                            })),
                        ),
                        _ => match from_binary(msg).unwrap() {
                            Cw20QueryMsg::TokenInfo {} => {
                                let balances: &HashMap<String, Uint128> =
                                match self.token_querier.balances.get(contract_addr) {
                                    Some(balances) => balances,
                                    None => {
                                        return SystemResult::Err(SystemError::InvalidRequest {
                                            error: format!(
                                                "No balance info exists for the contract {contract_addr}"
                                            ),
                                            request: msg.as_slice().into(),
                                        })
                                    }
                                };

                                let mut total_supply = Uint128::zero();

                                for balance in balances {
                                    total_supply += *balance.1;
                                }

                                SystemResult::Ok(ContractResult::Ok(
                                    to_binary(&TokenInfoResponse {
                                        name: "mAAPL".to_string(),
                                        symbol: "mAAPL".to_string(),
                                        decimals: 8,
                                        total_supply,
                                    })
                                    .unwrap(),
                                ))
                            }
                            Cw20QueryMsg::Balance { address } => {
                                let balances: &HashMap<String, Uint128> =
                                match self.token_querier.balances.get(contract_addr) {
                                    Some(balances) => balances,
                                    None => {
                                        return SystemResult::Err(SystemError::InvalidRequest {
                                            error: format!(
                                                "No balance info exists for the contract {contract_addr}"
                                            ),
                                            request: msg.as_slice().into(),
                                        })
                                    }
                                };

                                let balance = match balances.get(&address) {
                                    Some(v) => *v,
                                    None => {
                                        return SystemResult::Ok(ContractResult::Ok(
                                            to_binary(&Cw20BalanceResponse {
                                                balance: Uint128::zero(),
                                            })
                                            .unwrap(),
                                        ));
                                    }
                                };

                                SystemResult::Ok(ContractResult::Ok(
                                    to_binary(&Cw20BalanceResponse { balance }).unwrap(),
                                ))
                            }

                            _ => panic!("DO NOT ENTER HERE"),
                        },
                    },
                }
            }
            QueryRequest::Wasm(WasmQuery::ContractInfo { .. }) => {
                let mut contract_info_response = ContractInfoResponse::default();
                contract_info_response.code_id = 0u64;
                contract_info_response.creator = "creator".to_string();
                contract_info_response.admin = Some("creator".to_string());

                SystemResult::Ok(ContractResult::Ok(
                    to_binary(&contract_info_response).unwrap(),
                ))
            }
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier) -> Self {
        WasmMockQuerier {
            base,
            token_querier: TokenQuerier::default(),
            pool_factory_querier: PoolFactoryQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    // configure the pair
    pub fn with_pool_factory(
        &mut self,
        pairs: &[(&String, &PairInfo)],
        native_token_decimals: &[(String, u8)],
    ) {
        self.pool_factory_querier = PoolFactoryQuerier::new(pairs, native_token_decimals);
    }

    pub fn with_balance(&mut self, balances: &[(&String, Vec<Coin>)]) {
        for (addr, balance) in balances {
            self.base.update_balance(addr.to_string(), balance.clone());
        }
    }
}

impl WasmMockTrioQuerier {
    pub fn new(base: MockQuerier) -> Self {
        WasmMockTrioQuerier {
            base,
            token_querier: TokenQuerier::default(),
            pool_factory_querier: PoolFactoryQuerier::default(),
        }
    }

    // configure the mint whitelist mock querier
    pub fn with_token_balances(&mut self, balances: &[(&String, &[(&String, &Uint128)])]) {
        self.token_querier = TokenQuerier::new(balances);
    }

    // configure the pair
    pub fn with_pool_factory(
        &mut self,
        pairs: &[(&String, &PairInfo)],
        native_token_decimals: &[(String, u8)],
    ) {
        self.pool_factory_querier = PoolFactoryQuerier::new(pairs, native_token_decimals);
    }

    pub fn with_balance(&mut self, balances: &[(&String, Vec<Coin>)]) {
        for (addr, balance) in balances {
            self.base.update_balance(addr.to_string(), balance.clone());
        }
    }
}

#[cfg(test)]
mod mock_exception {
    use cosmwasm_std::Binary;

    use super::*;

    #[test]
    fn raw_query_err() {
        let deps = mock_dependencies(&[]);
        assert_eq!(
            deps.querier.raw_query(&[]),
            SystemResult::Err(SystemError::InvalidRequest {
                error: "Parsing query request: Error parsing into type cosmwasm_std::query::QueryRequest<cosmwasm_std::results::empty::Empty>: EOF while parsing a JSON value.".to_string(),
                request: Binary(vec![]),
            })
        );
    }

    #[test]
    fn none_factory_pair_will_err() {
        let deps = mock_dependencies(&[]);

        let msg = to_binary(&FactoryQueryMsg::Pair {
            asset_infos: [
                AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                AssetInfo::NativeToken {
                    denom: "ulunc".to_string(),
                },
            ],
        })
        .unwrap();
        assert_eq!(
            deps.querier
                .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: "contract0000".to_string(),
                    msg: msg.clone(),
                })),
            SystemResult::Err(SystemError::InvalidRequest {
                error: "No pair info exists".to_string(),
                request: msg,
            })
        )
    }

    #[test]
    fn none_tokens_info_will_err() {
        let deps = mock_dependencies(&[]);

        let msg = to_binary(&Cw20QueryMsg::TokenInfo {}).unwrap();

        assert_eq!(
            deps.querier
                .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: "token0000".to_string(),
                    msg: msg.clone(),
                })),
            SystemResult::Err(SystemError::InvalidRequest {
                error: "No balance info exists for the contract token0000".to_string(),
                request: msg,
            })
        )
    }

    #[test]
    fn none_tokens_balance_will_err() {
        let deps = mock_dependencies(&[]);

        let msg = to_binary(&Cw20QueryMsg::Balance {
            address: "address0000".to_string(),
        })
        .unwrap();

        assert_eq!(
            deps.querier
                .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: "token0000".to_string(),
                    msg: msg.clone(),
                })),
            SystemResult::Err(SystemError::InvalidRequest {
                error: "No balance info exists for the contract token0000".to_string(),
                request: msg,
            })
        )
    }

    #[test]
    #[should_panic]
    fn none_tokens_minter_will_panic() {
        let deps = mock_dependencies(&[]);

        let msg = to_binary(&Cw20QueryMsg::Minter {}).unwrap();

        deps.querier
            .handle_query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: "token0000".to_string(),
                msg,
            }));
    }
}

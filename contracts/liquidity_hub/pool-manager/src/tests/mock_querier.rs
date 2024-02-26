use std::collections::HashMap;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::panic;

use cosmwasm_std::testing::{MockQuerier, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractInfoResponse, ContractResult, Empty,
    OwnedDeps, Querier, QuerierResult, QueryRequest, SystemError, SystemResult, Uint128, WasmQuery, CodeInfoResponse, HexBinary, Addr,
};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use white_whale_std::pool_network::temp_mock_api::MockSimpleApi;
use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use white_whale_std::pool_network::asset::{Asset, AssetInfo, PairInfo, PairType, TrioInfo};
use white_whale_std::pool_network::factory::{NativeTokenDecimalsResponse, QueryMsg as FactoryQueryMsg};
use white_whale_std::pool_network::pair::{PoolResponse as PairPoolResponse, QueryMsg as PairQueryMsg, self};
use white_whale_std::pool_network::pair::{ReverseSimulationResponse, SimulationResponse};
use white_whale_std::pool_network::trio;
use white_whale_std::pool_network::trio::{PoolResponse as TrioPoolResponse, QueryMsg as TrioQueryMsg};
/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
/// this uses our CustomQuerier.
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockSimpleApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        storage: MockStorage::default(),
        api: MockSimpleApi::default(),
        querier: custom_querier,
        custom_query_type: PhantomData,
    }
}

pub struct WasmMockQuerier {
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
    // Key is the pool_identifer
    for (key, pair) in pairs.iter() {
        pairs_map.insert(key.to_string(), (**pair).clone());
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

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                match from_binary(msg) {
                    Ok(white_whale_std::pool_manager::QueryMsg::Pair { pair_identifier }) => {
                        
                        match self
                            .pool_factory_querier
                            .pairs
                            .get(&pair_identifier)
                        {
                            Some(v) => SystemResult::Ok(ContractResult::Ok(to_binary(v).unwrap())),
                            None => SystemResult::Err(SystemError::InvalidRequest {
                                error: "No pair info exists".to_string(),
                                request: msg.as_slice().into(),
                            }),
                        }
                    }
                    Ok(white_whale_std::pool_manager::QueryMsg::NativeTokenDecimals { denom }) => {
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
                       
                        Ok(white_whale_std::pool_manager::QueryMsg::Simulation { offer_asset, ask_asset, pair_identifier }) => {
                            SystemResult::Ok(ContractResult::from(to_binary(&SimulationResponse {
                                return_amount: offer_asset.amount,
                                swap_fee_amount: Uint128::zero(),
                                spread_amount: Uint128::zero(),
                                protocol_fee_amount: Uint128::zero(),
                                burn_fee_amount: Uint128::zero(),
                            })))
                        }
                        Ok(white_whale_std::pool_manager::QueryMsg::ReverseSimulation { ask_asset, offer_asset, pair_identifier }) => SystemResult::Ok(
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
                                println!("balances: {:?}", self.token_querier.balances);

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

        let msg = to_binary(&white_whale_std::pool_manager::QueryMsg::Pair {
            pair_identifier: "pair0000".to_string(),
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
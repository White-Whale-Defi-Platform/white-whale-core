// use std::collections::HashMap;

// use cosmwasm_std::{DepsMut, Env, Addr, Uint128, Decimal, Response, StdError, Deps, CosmosMsg, WasmMsg, to_binary, StdResult, Coin, MessageInfo};
// use cw20::Cw20ExecuteMsg;
// use white_whale::{pool_network::{asset::{AssetInfo, Asset}}, pool_manager::{SwapOperation, ExecuteMsg, NPairInfo}};

// use crate::{ContractError, state::{MANAGER_CONFIG, get_pair_by_identifier, Config}};

// fn assert_operations(operations: &[SwapOperation]) -> Result<(), ContractError> {
//     let mut ask_asset_map: HashMap<String, bool> = HashMap::new();
//     for operation in operations.iter() {
//         let (offer_asset, ask_asset, _pool_identifier) = match operation {
//             SwapOperation::WhaleSwap {
//                 token_in_info: offer_asset_info,
//                 token_out_info: ask_asset_info,
//                 pool_identifier,
//             } => (offer_asset_info.clone(), ask_asset_info.clone(), pool_identifier.clone()),
//         };

//         ask_asset_map.remove(&offer_asset.to_string());
//         ask_asset_map.insert(ask_asset.to_string(), true);
//     }

//     if ask_asset_map.keys().len() != 1 {
//         return Err(ContractError::MultipleOutputToken {});
//     }

//     Ok(())
// }

// pub fn execute_swap_operations(
//     deps: DepsMut,
//     env: Env,
//     sender: Addr,
//     operations: Vec<SwapOperation>,
//     minimum_receive: Option<Uint128>,
//     to: Option<Addr>,
//     max_spread: Option<Decimal>,
// ) -> Result<Response, ContractError> {
//     let operations_len = operations.len();
//     if operations_len == 0 {
//         return Err(StdError::generic_err("Must provide swap operations to execute").into());
//     }

//     // Assert the operations are properly set
//     assert_operations(&operations)?;

//     let to = if let Some(to) = to { to } else { sender };
//     let target_asset_info = operations
//         .last()
//         .ok_or_else(|| ContractError::Std(StdError::generic_err("Couldn't get swap operation")))?
//         .get_target_asset_info();

//     let mut operation_index = 0;
//     let mut messages: Vec<CosmosMsg> = operations
//         .into_iter()
//         .map(|op| {
//             operation_index += 1;
//             Ok(CosmosMsg::Wasm(WasmMsg::Execute {
//                 contract_addr: env.contract.address.to_string(),
//                 funds: vec![],
//                 msg: to_binary(&ExecuteMsg::ExecuteSwapOperation {
//                     operation: op,
//                     to: if operation_index == operations_len {
//                         Some(to.to_string())
//                     } else {
//                         None
//                     },
//                     max_spread,
//                 })?,
//             }))
//         })
//         .collect::<StdResult<Vec<CosmosMsg>>>()?;

//     // Execute minimum amount assertion
//     if let Some(minimum_receive) = minimum_receive {
//         let receiver_balance =
//             target_asset_info.query_balance(&deps.querier, deps.api, to.clone())?;

//         messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: env.contract.address.to_string(),
//             funds: vec![],
//             msg: to_binary(&ExecuteMsg::AssertMinimumReceive {
//                 asset_info: target_asset_info,
//                 prev_balance: receiver_balance,
//                 minimum_receive,
//                 receiver: to.to_string(),
//             })?,
//         }))
//     }

//     Ok(Response::new().add_messages(messages))
// }

// /// Execute swap operation
// /// swap all offer asset to ask asset
// pub fn execute_swap_operation(
//     deps: DepsMut,
//     env: Env,
//     info: MessageInfo,
//     operation: SwapOperation,
//     to: Option<String>,
//     max_spread: Option<Decimal>,
// ) -> Result<Response, ContractError> {
//     if env.contract.address != info.sender {
//         return Err(ContractError::Unauthorized {});
//     }

//     let messages: Vec<CosmosMsg> = match operation {
//         SwapOperation::WhaleSwap {
//             token_in_info,
//             token_out_info,
//             pool_identifier,
//         } => {
//             let _config: Config = MANAGER_CONFIG.load(deps.as_ref().storage)?;
//             let pair_info: NPairInfo = get_pair_by_identifier(
//                 &deps.as_ref(),
//                 pool_identifier.clone(),
//             )?;
//             let mut offer_asset: Asset = Asset {
//                 info: token_in_info.clone(),
//                 amount: Uint128::zero(),
//             };
//             // Return the offer_asset from pair_info.assets that matches token_in_info
//             for asset in pair_info.assets {
//                 if asset.info.equal(&token_in_info) {
//                     offer_asset = asset;
//                 }
//             }

//             vec![asset_into_swap_msg(
//                 deps.as_ref(),
//                 env.contract.address,
//                 offer_asset,
//                 token_out_info,
//                 pool_identifier,
//                 max_spread,
//                 to,
//             )?]
//         }
//     };

//     Ok(Response::new().add_messages(messages))
// }

// pub fn asset_into_swap_msg(
//     _deps: Deps,
//     pair_contract: Addr,
//     offer_asset: Asset,
//     ask_asset: AssetInfo,
//     pair_identifier: String,
//     max_spread: Option<Decimal>,
//     to: Option<String>,
// ) -> Result<CosmosMsg, ContractError> {
//     match offer_asset.info.clone() {
//         AssetInfo::NativeToken { denom } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr: pair_contract.to_string(),
//             funds: vec![Coin {
//                 denom,
//                 amount: offer_asset.amount,
//             }],
//             msg: to_binary(&white_whale::pool_manager::ExecuteMsg::Swap {
//                 offer_asset,
//                 belief_price: None,
//                 max_spread,
//                 to,
//                 ask_asset,
//                 pair_identifier,
//             })?,
//         })),
//         AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
//             contract_addr,
//             funds: vec![],
//             msg: to_binary(&Cw20ExecuteMsg::Send {
//                 contract: pair_contract.to_string(),
//                 amount: offer_asset.amount,
//                 msg: to_binary(&white_whale::pool_manager::Cw20HookMsg::Swap {
//                     belief_price: None,
//                     max_spread,
//                     to,
//                     ask_asset,
//                     pair_identifier,
//                 })?,
//             })?,
//         })),
//     }
// }

// pub fn assert_minimum_receive(
//     deps: Deps,
//     asset_info: AssetInfo,
//     prev_balance: Uint128,
//     minimum_receive: Uint128,
//     receiver: Addr,
// ) -> Result<Response, ContractError> {
//     let receiver_balance = asset_info.query_balance(&deps.querier, deps.api, receiver)?;
//     let swap_amount = receiver_balance.checked_sub(prev_balance)?;

//     if swap_amount < minimum_receive {
//         return Err(ContractError::MinimumReceiveAssertion {
//             minimum_receive,
//             swap_amount,
//         });
//     }

//     Ok(Response::default())
// }
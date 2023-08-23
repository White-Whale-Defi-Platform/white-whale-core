use classic_bindings::TerraQuery;
use cosmwasm_std::{
    Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo, Response, to_binary, WasmMsg,
};
use cw20::Cw20ExecuteMsg;

use white_whale::pool_network;
use white_whale::pool_network::asset::{Asset, AssetInfo, PairInfo};
use white_whale::pool_network::pair::ExecuteMsg as PairExecuteMsg;
use white_whale::pool_network::querier::{query_balance, query_pair_info, query_token_balance};
use white_whale::pool_network::router::SwapOperation;

use crate::error::ContractError;
use crate::state::{Config, CONFIG};

/// Execute swap operation
/// swap all offer asset to ask asset
pub fn execute_swap_operation(
    deps: DepsMut<TerraQuery>,
    env: Env,
    info: MessageInfo,
    operation: SwapOperation,
    to: Option<String>,
    max_spread: Option<Decimal>,
) -> Result<Response, ContractError> {
    if env.contract.address != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let messages: Vec<CosmosMsg> = match operation {
        SwapOperation::TerraSwap {
            offer_asset_info,
            ask_asset_info,
        } => {
            let config: Config = CONFIG.load(deps.as_ref().storage)?;
            let terraswap_factory = deps.api.addr_humanize(&config.terraswap_factory)?;
            let pair_info: PairInfo = query_pair_info(
                &deps.querier,
                terraswap_factory,
                &[offer_asset_info.clone(), ask_asset_info],
            )?;

            let amount = match offer_asset_info.clone() {
                AssetInfo::NativeToken { denom } => {
                    query_balance(&deps.querier, env.contract.address, denom)?
                }
                AssetInfo::Token { contract_addr } => query_token_balance(
                    &deps.querier,
                    deps.api.addr_validate(contract_addr.as_str())?,
                    env.contract.address,
                )?,
            };
            let offer_asset: Asset = Asset {
                info: offer_asset_info,
                amount,
            };

            vec![asset_into_swap_msg(
                deps.as_ref(),
                Addr::unchecked(pair_info.contract_addr),
                offer_asset,
                max_spread,
                to,
            )?]
        }
    };

    Ok(Response::new().add_messages(messages))
}

pub fn asset_into_swap_msg(
    deps: Deps<TerraQuery>,
    pair_contract: Addr,
    mut offer_asset: Asset,
    max_spread: Option<Decimal>,
    to: Option<String>,
) -> Result<CosmosMsg, ContractError> {
    match offer_asset.info.clone() {
        AssetInfo::NativeToken { denom } => {
            // deduct tax
            offer_asset.amount = offer_asset.deduct_tax(&deps.querier)?.amount;

            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_contract.to_string(),
                funds: vec![Coin {
                    denom,
                    amount: offer_asset.amount,
                }],
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset,
                    belief_price: None,
                    max_spread,
                    to,
                })?,
            }))
        }
        AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                contract: pair_contract.to_string(),
                amount: offer_asset.amount,
                msg: to_binary(&pool_network::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread,
                    to,
                })?,
            })?,
        })),
    }
}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::pool_network::frontend_helper::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, TempState,
};
use white_whale::traits::OptionDecimal;

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::reply;
use crate::reply::deposit_pair::DEPOSIT_PAIR_REPLY_ID;
use crate::state::{CONFIG, TEMP_STATE};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-frontend-helper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        incentive_factory_addr: deps.api.addr_validate(&msg.incentive_factory)?,
        owner: deps.api.addr_validate(info.sender.as_ref())?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {
            pair_address,
            assets,
            slippage_tolerance,
            unbonding_duration,
        } => {
            TEMP_STATE.save(
                deps.storage,
                &TempState {
                    unbonding_duration,
                    receiver: info.sender.clone(),
                    pair_addr: deps.api.addr_validate(&pair_address)?,
                },
            )?;

            let transfer_token_msgs = assets
                .iter()
                .filter_map(|asset| match asset.info.clone() {
                    AssetInfo::NativeToken { .. } => None,
                    AssetInfo::Token { contract_addr } => Some((asset.amount, contract_addr)),
                })
                .map(|(token_amount, token_contract_addr)| {
                    // ensure that we have this token amount
                    let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                        token_contract_addr.clone(),
                        &cw20::Cw20QueryMsg::Allowance {
                            owner: info.sender.clone().into_string(),
                            spender: env.contract.address.clone().into_string(),
                        },
                    )?;

                    if allowance.allowance != token_amount {
                        return Err(ContractError::MissingToken {
                            asset: Asset {
                                info: AssetInfo::Token {
                                    contract_addr: token_contract_addr,
                                },
                                amount: token_amount,
                            },
                            current_allowance: allowance.allowance,
                        });
                    }

                    Ok::<_, ContractError>(vec![
                        WasmMsg::Execute {
                            contract_addr: token_contract_addr.clone(),
                            msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                                owner: info.sender.clone().into_string(),
                                recipient: env.contract.address.clone().into_string(),
                                amount: token_amount,
                            })?,
                            funds: vec![],
                        },
                        WasmMsg::Execute {
                            contract_addr: token_contract_addr,
                            msg: to_binary(&cw20::Cw20ExecuteMsg::IncreaseAllowance {
                                spender: pair_address.clone(),
                                amount: token_amount,
                                expires: None,
                            })?,
                            funds: vec![],
                        },
                    ])
                })
                .collect::<Result<Vec<_>, _>>()?
                .concat();

            // send request to deposit
            Ok(Response::default()
                .add_attributes(vec![
                    ("action", "deposit".to_string()),
                    ("pair_address", pair_address.clone()),
                    ("unbonding_duration", unbonding_duration.to_string()),
                    ("slippage_tolerance", slippage_tolerance.to_string()),
                ])
                .add_messages(transfer_token_msgs)
                .add_submessage(SubMsg {
                    id: DEPOSIT_PAIR_REPLY_ID,
                    reply_on: cosmwasm_std::ReplyOn::Always,
                    gas_limit: None,
                    msg: WasmMsg::Execute {
                        contract_addr: pair_address,
                        msg: to_binary(
                            &white_whale::pool_network::pair::ExecuteMsg::ProvideLiquidity {
                                assets,
                                slippage_tolerance,
                                receiver: None,
                            },
                        )?,
                        funds: info.funds,
                    }
                    .into(),
                }))
        }
        ExecuteMsg::UpdateConfig {
            incentive_factory_addr,
            owner,
        } => {
            let mut config = CONFIG.load(deps.storage)?;
            if config.owner != info.sender {
                return Err(ContractError::Unauthorized {});
            }

            if let Some(owner) = owner {
                config.owner = deps.api.addr_validate(&owner)?;
            }

            if let Some(incentive_factory_addr) = incentive_factory_addr {
                config.incentive_factory_addr = deps.api.addr_validate(&incentive_factory_addr)?;
            }

            CONFIG.save(deps.storage, &config)?;

            Ok(Response::default().add_attributes(vec![
                ("action", "update_config".to_string()),
                ("owner", config.owner.to_string()),
                (
                    "incentive_factory_addr",
                    config.incentive_factory_addr.to_string(),
                ),
            ]))
        }
    }
}

/// Handles reply messages from submessages sent out by the frontend contract.
#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        DEPOSIT_PAIR_REPLY_ID => reply::deposit_pair::deposit_pair(deps, env, msg),
        id => Err(ContractError::UnknownReplyId { id }),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&CONFIG.load(deps.storage)?)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

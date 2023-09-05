use cosmwasm_std::entry_point;
use cosmwasm_std::{coins, from_binary};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, OverflowError,
    Response, StdError, StdResult, Storage, Uint128, WasmMsg,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Item;
use cw_utils::one_coin;
use semver::Version;

#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use white_whale::pool_network::asset::is_factory_token;
use white_whale::pool_network::asset::{Asset, AssetInfo};
#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::{Coin, MsgBurn, MsgMint};
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::{Coin, MsgBurn, MsgMint};
use white_whale::vault_manager::{
    CallbackMsg, Cw20HookMsg, ExecuteMsg, InstantiateMsg, ManagerConfig, MigrateMsg,
    PaybackAmountResponse, QueryMsg,
};

use crate::error::ContractError;
use crate::state::{
    ALL_TIME_BURNED_FEES, COLLECTED_PROTOCOL_FEES, LOAN_COUNTER, MANAGER_CONFIG, OWNER, VAULTS,
};
use crate::{manager, vault};

// version info for migration info
const CONTRACT_NAME: &str = "ww-vault-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg.vault_creation_fee.info {
        AssetInfo::Token { .. } => {
            return Err(StdError::generic_err("Vault creation fee must be native token").into());
        }
        AssetInfo::NativeToken { .. } => {}
    }

    let manager_config = ManagerConfig {
        lp_token_type: msg.lp_token_type,
        fee_collector_addr: deps.api.addr_validate(&msg.fee_collector_addr)?,
        vault_creation_fee: msg.vault_creation_fee,
        flash_loan_enabled: true,
        deposit_enabled: true,
        withdraw_enabled: true,
    };
    MANAGER_CONFIG.save(deps.storage, &manager_config)?;

    //todo ownership proposal stuff to change ownership of the contract
    OWNER.save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", msg.owner),
        ("lp_token_type", manager_config.lp_token_type.to_string()),
        (
            "fee_collector_addr",
            manager_config.fee_collector_addr.into_string(),
        ),
        (
            "vault_creation_fee",
            manager_config.vault_creation_fee.to_string(),
        ),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateVault { asset_info, fees } => {
            manager::commands::create_vault(deps, env, info, asset_info, fees)
        }
        ExecuteMsg::RemoveVault { asset_info } => {
            todo!();
            // if let Ok(None) = VAULTS.may_load(deps.storage, asset_info.get_reference()) {
            //     return Err(ContractError::NonExistentVault {});
            // }
            //
            // VAULTS.remove(deps.storage, asset_info.get_reference());

            Ok(Response::new().add_attributes(vec![("method", "remove_vault")]))
        }
        ExecuteMsg::UpdateVaultFees {
            vault_asset_info,
            vault_fee,
        } => manager::commands::update_vault_fees(deps, info, vault_asset_info, vault_fee),
        ExecuteMsg::UpdateManagerConfig {
            fee_collector_addr,
            vault_creation_fee,
            cw20_lp_code_id,
            flash_loan_enabled,
            deposit_enabled,
            withdraw_enabled,
        } => manager::commands::update_manager_config(
            deps,
            info,
            fee_collector_addr,
            vault_creation_fee,
            cw20_lp_code_id,
            flash_loan_enabled,
            deposit_enabled,
            withdraw_enabled,
        ),
        ExecuteMsg::Deposit { asset } => vault::commands::deposit(&deps, &env, &info, &asset),
        ExecuteMsg::Withdraw {} => {
            let lp_asset = AssetInfo::NativeToken {
                denom: one_coin(&info)?.denom,
            };

            // check if the vault exists
            let vault = VAULTS
                .idx
                .lp_asset
                .item(deps.storage, lp_asset.to_string())?
                .map_or_else(|| Err(ContractError::NonExistentVault {}), Ok)?
                .1;

            // validate that the asset sent is the token factory LP token
            let config = MANAGER_CONFIG.load(deps.storage)?;
            let lp_token_denom = match config.vault_creation_fee.info {
                AssetInfo::Token { .. } => String::new(),
                AssetInfo::NativeToken { denom } => denom,
            };

            if info.funds.len() != 1 || info.funds[0].denom != lp_token_denom {
                return Err(ContractError::Unauthorized {});
            }

            vault::commands::withdraw(
                deps,
                env,
                info.sender.into_string(),
                info.funds[0].amount,
                vault,
            )
        }
        ExecuteMsg::Receive(msg) => {
            // check if it's a cw20 lp asset executing this callback
            let vault = VAULTS
                .idx
                .lp_asset
                .item(deps.storage, info.sender.to_string())?
                .map_or_else(|| Err(ContractError::Unauthorized {}), Ok)?
                .1;

            match from_binary(&msg.msg)? {
                Cw20HookMsg::Withdraw {} => {
                    vault::commands::withdraw(deps, env, msg.sender, msg.amount, vault)
                }
            }
        }
        ExecuteMsg::Callback(msg) => {
            todo!();
            callback(deps, env, info, msg)
        }
        ExecuteMsg::FlashLoan { assets, msgs } => {
            todo!();
            Ok(Response::default())
        }
        ExecuteMsg::NextLoan {
            initiator,
            source_vault_asset_info,
            payload,
            to_loan,
            loaned_assets,
        } => {
            todo!();
            Ok(Response::default())
        }
        ExecuteMsg::CompleteLoan {
            initiator,
            loaned_assets,
        } => {
            todo!();
            Ok(Response::default())
        }
    }
}

pub fn complete_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    initiator: Addr,
    assets: Vec<(String, Asset)>,
) -> Result<Response, ContractError> {
    // check that the contract itself is executing this message
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    // pay back loans and profit
    let messages: Vec<Vec<CosmosMsg>> = assets
        .into_iter()
        .map(|(vault, loaned_asset)| {
            let payback_amount: PaybackAmountResponse = deps.querier.query_wasm_smart(
                vault.clone(),
                &white_whale::vault_network::vault::QueryMsg::GetPaybackAmount {
                    amount: loaned_asset.amount,
                },
            )?;

            // calculate amount router has after performing flash-loan
            let final_amount = match &loaned_asset.info {
                AssetInfo::NativeToken { denom } => {
                    let amount = deps
                        .querier
                        .query_balance(env.contract.address.clone(), denom)?;

                    amount.amount
                }
                AssetInfo::Token { contract_addr } => {
                    let res: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                        contract_addr,
                        &cw20::Cw20QueryMsg::Balance {
                            address: env.contract.address.clone().into_string(),
                        },
                    )?;

                    res.balance
                }
            };

            let profit_amount = final_amount
                .checked_sub(payback_amount.payback_amount.amount.clone())
                .map_err(|_| ContractError::Unauthorized {})
                .unwrap();

            let mut response_messages: Vec<CosmosMsg> = vec![];
            let payback_loan_msg: StdResult<CosmosMsg> = match loaned_asset.info.clone() {
                AssetInfo::NativeToken { denom } => Ok(BankMsg::Send {
                    to_address: vault,
                    amount: coins(payback_amount.payback_amount.amount.u128(), denom),
                }
                .into()),
                AssetInfo::Token { contract_addr } => Ok(WasmMsg::Execute {
                    contract_addr,
                    funds: vec![],
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: vault,
                        amount: payback_amount.payback_amount.amount,
                    })?,
                }
                .into()),
            };

            response_messages.push(payback_loan_msg?);

            // add profit message if non-zero profit
            if !profit_amount.is_zero() {
                let profit_payback_msg: StdResult<CosmosMsg> = match loaned_asset.info {
                    AssetInfo::NativeToken { denom } => Ok(BankMsg::Send {
                        to_address: initiator.clone().into_string(),
                        amount: coins(profit_amount.u128(), denom),
                    }
                    .into()),
                    AssetInfo::Token { contract_addr } => Ok(WasmMsg::Execute {
                        contract_addr,
                        funds: vec![],
                        msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                            recipient: initiator.clone().into_string(),
                            amount: profit_amount,
                        })?,
                    }
                    .into()),
                };

                response_messages.push(profit_payback_msg?);
            }

            Ok(response_messages)
        })
        .collect::<StdResult<Vec<Vec<_>>>>()?;

    Ok(Response::new()
        .add_messages(messages.concat())
        .add_attributes(vec![("method", "complete_loan")]))
}

#[allow(clippy::too_many_arguments)]
pub fn next_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mut payload: Vec<CosmosMsg>,
    initiator: Addr,
    source_vault: String,
    source_vault_asset: AssetInfo,
    to_loan: Vec<(String, Asset)>,
    loaned_assets: Vec<(String, Asset)>,
) -> Result<Response, ContractError> {
    // check that the source vault is executing this message and it is a vault created by the WW vault factory
    let config = MANAGER_CONFIG.load(deps.storage)?;

    let Some(queried_vault) = deps.querier.query_wasm_smart::<Option<String>>(
        Addr::unchecked("asjdfkjahsdjkf"),
        &white_whale::vault_network::vault_factory::QueryMsg::Vault {
            asset_info: source_vault_asset,
        },
    )? else {
        return Err(ContractError::Unauthorized {});
    };

    let validated_source_vault = deps.api.addr_validate(&source_vault)?;

    if info.sender != validated_source_vault
        || deps.api.addr_validate(&queried_vault)? != validated_source_vault
    {
        return Err(ContractError::Unauthorized {});
    }

    let messages = match to_loan.split_first() {
        Some(((vault, asset), loans)) => {
            // loan next asset
            vec![WasmMsg::Execute {
                contract_addr: vault.clone(),
                funds: vec![],
                msg: to_binary(&white_whale::vault_network::vault::ExecuteMsg::FlashLoan {
                    amount: asset.amount,
                    msg: to_binary(&ExecuteMsg::NextLoan {
                        initiator,
                        source_vault_asset_info: asset.info.clone(),
                        to_loan: loans.to_vec(),
                        payload,
                        loaned_assets,
                    })?,
                })?,
            }
            .into()]
        }
        None => {
            payload.push(
                // pay back all the loans at the end
                WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    funds: vec![],
                    msg: to_binary(&ExecuteMsg::CompleteLoan {
                        initiator,
                        loaned_assets,
                    })?,
                }
                .into(),
            );

            payload
        }
    };

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![("method", "next_loan")]))
}

pub fn flash_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    // check that flash loans are enabled
    let config = MANAGER_CONFIG.load(deps.storage)?;
    if !config.flash_loan_enabled {
        return Err(ContractError::Unauthorized {});
    }

    // increment loan counter
    LOAN_COUNTER.update::<_, StdError>(deps.storage, |c| {
        Ok(c.checked_add(1)
            .ok_or_else(|| OverflowError::new(cosmwasm_std::OverflowOperation::Add, c, 1))?)
    })?;

    // store current balance for after trade profit check
    let old_balance = match config.vault_creation_fee.info.clone() {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address.clone(), denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let resp: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.clone().into_string(),
                },
            )?;
            resp.balance
        }
    };

    let mut messages: Vec<CosmosMsg> = vec![];

    // create message to send funds to sender if cw20 token
    if let AssetInfo::Token { contract_addr } = config.vault_creation_fee.info.clone() {
        let loan_msg = WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: info.sender.clone().into_string(),
                amount,
            })?,
            funds: vec![],
        }
        .into();
        messages.push(loan_msg);
    };

    // get funds to send to callback (if native token then send in the callback msg)
    let callback_funds = match config.vault_creation_fee.info.clone() {
        AssetInfo::Token { .. } => vec![],
        AssetInfo::NativeToken { denom } => coins(amount.u128(), denom),
    };

    // add callback msg to messages
    messages.push(
        WasmMsg::Execute {
            contract_addr: info.sender.into_string(),
            msg,
            funds: callback_funds,
        }
        .into(),
    );

    // call after trade msg
    messages.push(
        WasmMsg::Execute {
            contract_addr: env.contract.address.into_string(),
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterTrade {
                old_balance,
                loan_amount: amount,
            }))?,
            funds: vec![],
        }
        .into(),
    );

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("method", "flash_loan"),
        ("amount", &amount.to_string()),
    ]))
}

pub fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, ContractError> {
    // callback can only be called by contract
    if info.sender != env.contract.address {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        CallbackMsg::AfterTrade {
            old_balance,
            loan_amount,
        } => after_trade(deps, env, old_balance, loan_amount),
    }
}

pub fn after_trade(
    deps: DepsMut,
    env: Env,
    old_balance: Uint128,
    loan_amount: Uint128,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;

    // query balance
    let new_balance = match config.vault_creation_fee.info.clone() {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address.into_string(), denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let res: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.into_string(),
                },
            )?;
            res.balance
        }
    };

    // check that balance is greater than expected
    let protocol_fee = Uint128::new(255554545);
    let flash_loan_fee = Uint128::try_from(config.vault_creation_fee.amount).unwrap();
    let burn_fee = config.vault_creation_fee.amount;

    let required_amount = old_balance
        .checked_add(protocol_fee)
        .unwrap()
        .checked_add(flash_loan_fee)
        .unwrap()
        .checked_add(burn_fee)
        .unwrap();

    if required_amount > new_balance {
        return Err(ContractError::Unauthorized {});
    }

    let profit = new_balance
        .checked_sub(old_balance)
        .unwrap()
        .checked_sub(protocol_fee)
        .unwrap()
        .checked_sub(flash_loan_fee)
        .unwrap()
        .checked_sub(burn_fee)
        .unwrap();

    // store fees
    store_fee(deps.storage, COLLECTED_PROTOCOL_FEES, protocol_fee).unwrap();

    // deduct loan counter
    LOAN_COUNTER
        .update::<_, StdError>(deps.storage, |c| Ok(c.saturating_sub(1)))
        .unwrap();

    let mut response = Response::new();
    if !burn_fee.is_zero() {
        let burn_asset = Asset {
            info: config.vault_creation_fee.info.clone(),
            amount: burn_fee,
        };

        store_fee(deps.storage, ALL_TIME_BURNED_FEES, burn_fee)?;

        response = response.add_message(burn_asset.into_burn_msg()?);
    }

    Ok(response.add_attributes(vec![
        ("method", "after_trade".to_string()),
        ("profit", profit.to_string()),
        ("protocol_fee", protocol_fee.to_string()),
        ("flash_loan_fee", flash_loan_fee.to_string()),
        ("burn_fee", burn_fee.to_string()),
    ]))
}

/// Stores a fee in the given fees_storage_item
pub fn store_fee(
    storage: &mut dyn Storage,
    fees_storage_item: Item<Asset>,
    fee: Uint128,
) -> StdResult<Asset> {
    fees_storage_item.update::<_, StdError>(storage, |mut fees| {
        fees.amount = fees.amount.checked_add(fee)?;
        Ok(fees)
    })
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use white_whale::migrate_guards::check_contract_name;

    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(ContractError::MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

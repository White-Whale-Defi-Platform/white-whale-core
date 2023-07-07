use cosmwasm_std::{Addr, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, from_binary, MessageInfo, Order, OverflowError, Response, StdError, StdResult, Storage, to_binary, Uint128, Uint256, wasm_execute, WasmMsg};
use cosmwasm_std::entry_point;
use cw2::{get_contract_version, set_contract_version};
use cw20::{AllowanceResponse, BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use cw_storage_plus::{Item, Map};
use semver::Version;
use white_whale::fee::{Fee, VaultFee};
use white_whale::pool_network::asset::{Asset, AssetInfo, get_total_share, is_factory_token, MINIMUM_LIQUIDITY_AMOUNT};
use white_whale::traits::AssetReference;

use white_whale::vault_manager::{CallbackMsg, Cw20HookMsg, ExecuteMsg, InstantiateMsg, ManagerConfig, MigrateMsg, PaybackAmountResponse, QueryMsg, Vault};

#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::{Coin, MsgMint, MsgBurn};
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::{Coin, MsgMint, MsgBurn};
#[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
use white_whale::pool_network::asset::is_factory_token;
use cosmwasm_std::coins;

use crate::error::ContractError;
use crate::manager;
use crate::state::{ALL_TIME_BURNED_FEES, COLLECTED_PROTOCOL_FEES, LOAN_COUNTER, MANAGER_CONFIG, VAULTS};

// version info for migration info
const CONTRACT_NAME: &str = "ww-vault-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let manager_config = ManagerConfig {
        owner: deps.api.addr_validate(&msg.owner)?,
        token_id: msg.token_id,
        fee_collector_addr: deps.api.addr_validate(&msg.fee_collector_addr)?,
        vault_creation_fee: msg.vault_creation_fee,
        flash_loan_enabled: true,
        deposit_enabled: true,
        withdraw_enabled: true,
    };
    MANAGER_CONFIG.save(deps.storage, &manager_config)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateVault { asset_info, fees, token_factory_lp } => manager::commands::create_vault(deps, env, info, asset_info, fees, token_factory_lp),
        ExecuteMsg::RemoveVault { asset_info } => {
            if let Ok(None) = VAULTS.may_load(deps.storage, asset_info.get_reference()) {
                return Err(ContractError::NonExistentVault {});
            }

            VAULTS.remove(deps.storage, asset_info.get_reference());

            Ok(Response::new().add_attributes(vec![("method", "remove_vault")]))

        },
        ExecuteMsg::UpdateVault { vault_asset_info, params } => {

            let mut vault = VAULTS.may_load(deps.storage, vault_asset_info.get_reference())?.unwrap();

            vault.fees = VaultFee {
                protocol_fee: Fee { share: Default::default() },
                flash_loan_fee: Fee { share: Default::default() },
                burn_fee: Fee { share: Default::default() },
            };

            vault.fees = VaultFee {
                protocol_fee: Fee { share: Default::default() },
                flash_loan_fee: Fee { share: Default::default() },
                burn_fee: Fee { share: Default::default() },
            };

            Ok(Response::new()
                    .add_attribute("method", "update_vault_config"))

        },
        ExecuteMsg::UpdateManagerConfig { owner, fee_collector_addr, token_id } => {
            let new_config = MANAGER_CONFIG.update::<_, ContractError>(deps.storage, |mut config| {
                if let Some(new_owner) = owner {
                    config.owner = deps.api.addr_validate(&new_owner)?;
                };

                if let Some(new_fee_collector_addr) = fee_collector_addr {
                    config.fee_collector_addr = deps.api.addr_validate(&new_fee_collector_addr)?;
                }

                if let Some(new_token_id) = token_id {
                    config.token_id = new_token_id;
                }

                Ok(config)
            })?;

            Ok(Response::new().add_attributes(vec![
                ("method", "update_config"),
                ("owner", &new_config.owner.into_string()),
                (
                    "fee_collector_addr",
                    &new_config.fee_collector_addr.into_string(),
                ),
                ("token_id", &new_config.token_id.to_string()),
            ]))

        },
        ExecuteMsg::Deposit { asset } => {
            let config = MANAGER_CONFIG.load(deps.storage)?;

            // check that deposits are enabled
            if !config.deposit_enabled {
                return Err(ContractError::Unauthorized {});
            }

            // check that we are not currently in a flash-loan
            if LOAN_COUNTER.load(deps.storage)? != 0 {
                // more than 0 loans is being performed currently
                return Err(ContractError::Unauthorized {});
            }

            // check that user sent assets they said they did
            let sent_funds = match config.vault_creation_fee.info.clone() {
                AssetInfo::NativeToken { denom } => info
                    .funds
                    .iter()
                    .filter(|c| c.denom == denom)
                    .map(|c| c.amount)
                    .sum::<Uint128>(),
                AssetInfo::Token { contract_addr } => {
                    let allowance: AllowanceResponse = deps.querier.query_wasm_smart(
                        contract_addr,
                        &cw20::Cw20QueryMsg::Allowance {
                            owner: info.sender.clone().into_string(),
                            spender: env.contract.address.clone().into_string(),
                        },
                    )?;

                    allowance.allowance
                }
            };
            if sent_funds != asset.amount.clone() {
                return Err(ContractError::Unauthorized {});
            }

            let mut messages: Vec<CosmosMsg> = vec![];
            // add cw20 transfer message if needed
            if let AssetInfo::Token { contract_addr } = config.vault_creation_fee.info.clone() {
                messages.push(
                    WasmMsg::Execute {
                        contract_addr,
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: info.sender.clone().into_string(),
                            recipient: env.contract.address.clone().into_string(),
                            amount: Uint128::new(213),
                        })?,
                        funds: vec![],
                    }
                        .into(),
                )
            }

            let liquidity_asset = match config.vault_creation_fee.info.clone() {
                AssetInfo::Token { contract_addr } => contract_addr,
                AssetInfo::NativeToken { denom } => denom,
            };

            // mint LP token for the sender
            let total_share = get_total_share(&deps.as_ref(), liquidity_asset.clone())?;

            let lp_amount = if total_share.is_zero() {
                // Make sure at least MINIMUM_LIQUIDITY_AMOUNT is deposited to mitigate the risk of the first
                // depositor preventing small liquidity providers from joining the vault
                let share = asset.amount.clone()
                    .checked_sub(MINIMUM_LIQUIDITY_AMOUNT)
                    .map_err(|_| ContractError::InvalidVaultCreationFee{ amount: Default::default(), expected: Default::default() })?;

                messages.append(&mut mint_lp_token_msg(
                    liquidity_asset.clone(),
                    env.contract.address.to_string(),
                    env.contract.address.to_string(),
                    MINIMUM_LIQUIDITY_AMOUNT,
                )?);

                // share should be above zero after subtracting the MINIMUM_LIQUIDITY_AMOUNT
                if share.is_zero() {
                    return Err(ContractError::Unauthorized {});
                }

                share
            } else {
                // If the asset is native token, the balance has already increased in the vault
                // To calculate it properly we should subtract user deposit from the vault.
                // If the asset is a cw20 token, the balance has not changed yet so we don't need to subtract it
                let deposit_amount = match config.vault_creation_fee.info.clone() {
                    AssetInfo::NativeToken { .. } => Uint128::zero(),
                    AssetInfo::Token { .. } => Uint128::zero(),
                };

                // return based on a share of the total pool
                let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
                let total_deposits = config
                    .vault_creation_fee.info.clone()
                    .query_pool(&deps.querier, deps.api, env.contract.address.clone())?
                    .checked_sub(collected_protocol_fees.amount).unwrap()
                    .checked_sub(deposit_amount).unwrap();

                Uint128::zero()
                    .checked_mul(total_share).unwrap()
                    .checked_div(total_deposits).unwrap()
            };

            // mint LP token to sender
            messages.append(&mut mint_lp_token_msg(
                liquidity_asset,
                info.sender.into_string(),
                env.contract.address.to_string(),
                lp_amount,
            )?);

            Ok(Response::new()
                .add_messages(messages)
                .add_attributes(vec![("method", "deposit"), ("amount", &Uint128::zero().to_string())]))

        },
        ExecuteMsg::Withdraw { } => {
            // validate that the asset sent is the token factory LP token
            let config = MANAGER_CONFIG.load(deps.storage)?;
            let lp_token_denom = match config.vault_creation_fee.info {
                AssetInfo::Token { .. } => String::new(),
                AssetInfo::NativeToken { denom } => denom,
            };

            if info.funds.len() != 1 || info.funds[0].denom != lp_token_denom {
                return Err(ContractError::Unauthorized {});
            }

            withdraw(deps, env, info.sender.into_string(), info.funds[0].amount)
        },
        ExecuteMsg::Receive(msg) => {
            // callback can only be called by liquidity token
            let config = MANAGER_CONFIG.load(deps.storage)?;

            let cw20_lp_token = match config.vault_creation_fee.info {
                AssetInfo::Token { contract_addr } => contract_addr,
                AssetInfo::NativeToken { .. } => return Err(ContractError::Unauthorized {}),
            };

            if info.sender != deps.api.addr_validate(cw20_lp_token.as_str())? {
                return Err(ContractError::Unauthorized {});
            }

            match from_binary(&msg.msg)? {
                Cw20HookMsg::Withdraw {} => withdraw(deps, env, msg.sender, msg.amount),
            }
        },
        ExecuteMsg::Callback(msg) => {
            callback(deps, env, info, msg)
        },
        ExecuteMsg::FlashLoan { assets, msgs } => {


            Ok(Response::default()) },
        ExecuteMsg::NextLoan { initiator, source_vault_asset_info, payload, to_loan, loaned_assets } => {


            Ok(Response::default()) },
        ExecuteMsg::CompleteLoan { initiator, loaned_assets } => {

            Ok(Response::default()) }
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
                .map_err(|_| ContractError::Unauthorized {}).unwrap();

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
            let resp: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
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
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
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
            let res: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
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
        .checked_add(protocol_fee).unwrap()
        .checked_add(flash_loan_fee).unwrap()
        .checked_add(burn_fee).unwrap();

    if required_amount > new_balance {
        return Err(ContractError::Unauthorized {});
    }

    let profit = new_balance
        .checked_sub(old_balance).unwrap()
        .checked_sub(protocol_fee).unwrap()
        .checked_sub(flash_loan_fee).unwrap()
        .checked_sub(burn_fee).unwrap();

    // store fees
    store_fee(deps.storage, COLLECTED_PROTOCOL_FEES, protocol_fee).unwrap();

    // deduct loan counter
    LOAN_COUNTER.update::<_, StdError>(deps.storage, |c| Ok(c.saturating_sub(1))).unwrap();

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


pub fn withdraw(
    deps: DepsMut,
    env: Env,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = MANAGER_CONFIG.load(deps.storage)?;

    // check that withdrawals are enabled
    if !config.withdraw_enabled {
        return Err(ContractError::Unauthorized {});
    }

    // parse sender
    let sender = deps.api.addr_validate(&sender)?;

    // calculate the size of vault and the amount of assets to withdraw
    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    let total_asset_amount = match &config.vault_creation_fee.info.clone() {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address.clone(), denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let balance: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.clone().into_string(),
                },
            )?;
            balance.balance
        }
    } // deduct protocol fees
        .checked_sub(collected_protocol_fees.amount).unwrap();

    let liquidity_asset = match config.vault_creation_fee.info.clone() {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom } => denom,
    };

    let total_share = get_total_share(&deps.as_ref(), liquidity_asset.clone())?;

    let withdraw_amount = Decimal::from_ratio(amount, total_share) * total_asset_amount;

    // create message to send back to user if cw20
    let messages: Vec<CosmosMsg> = vec![
        match config.vault_creation_fee.info {
            AssetInfo::NativeToken { denom } => BankMsg::Send {
                to_address: sender.into_string(),
                amount: coins(withdraw_amount.u128(), denom),
            }
                .into(),
            AssetInfo::Token { contract_addr } => WasmMsg::Execute {
                contract_addr,
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: sender.into_string(),
                    amount: withdraw_amount,
                })?,
                funds: vec![],
            }
                .into(),
        },
        burn_lp_asset_msg(liquidity_asset, env.contract.address.to_string(), amount)?,
    ];

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("method", "withdraw"),
        ("lp_amount", &amount.to_string()),
        ("asset_amount", &withdraw_amount.to_string()),
    ]))
}


/// Creates the Burn LP message
#[allow(unused_variables)]
fn burn_lp_asset_msg(
    liquidity_asset: String,
    sender: String,
    amount: Uint128,
) -> Result<CosmosMsg, ContractError> {
    #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
    if is_factory_token(liquidity_asset.as_str()) {
        Ok(<MsgBurn as Into<CosmosMsg>>::into(MsgBurn {
            sender,
            amount: Some(Coin {
                denom: liquidity_asset,
                amount: amount.to_string(),
            }),
        }))
    } else {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: liquidity_asset,
            msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
            funds: vec![],
        }))
    }
    #[cfg(all(not(feature = "token_factory"), not(feature = "osmosis_token_factory")))]
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_asset,
        msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
        funds: vec![],
    }))
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}


#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
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

    if storage_version <= Version::parse("1.0.9")? {
        // do some migration

        const VAULTSV109: Map<&[u8], Addr> = Map::new("vaults");

        // There are less than 10 vaults in each network at this point
        let vaults_v109 = VAULTSV109
            .range(deps.storage, None, None, Order::Ascending)
            .take(10)
            .collect::<Result<Vec<_>, _>>()?;

        vaults_v109
            .into_iter()
            .try_for_each(|(key, vault_addr)| -> Result<(), StdError> {
                // All the vaults created so far are using native tokens
                // try to parse reference as a denom value
                let asset_info = AssetInfo::NativeToken {
                    denom: String::from_utf8(key.clone())?,
                };

                VAULTS.save(deps.storage, &key, &Vault{
                    asset_info,
                    asset_info_reference: vec![],
                    lp_asset: AssetInfo::NativeToken { denom: "".to_string() },
                    fees: VaultFee {
                        protocol_fee: Fee { share: Default::default() },
                        flash_loan_fee: Fee { share: Default::default() },
                        burn_fee: Fee { share: Default::default() },
                    },
                })?;

                Ok(())
            })?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}


/// Creates the Mint LP message
#[allow(unused_variables)]
fn mint_lp_token_msg(
    liquidity_asset: String,
    recipient: String,
    sender: String,
    amount: Uint128,
) -> Result<Vec<CosmosMsg>, ContractError> {
    #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
    if is_factory_token(liquidity_asset.as_str()) {
        let mut messages = vec![];
        messages.push(<MsgMint as Into<CosmosMsg>>::into(MsgMint {
            sender: sender.clone(),
            amount: Some(Coin {
                denom: liquidity_asset.clone(),
                amount: amount.to_string(),
            }),
        }));

        if sender != recipient {
            messages.push(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: recipient,
                amount: coins(amount.u128(), liquidity_asset.as_str()),
            }));
        }

        Ok(messages)
    } else {
        Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: liquidity_asset,
            msg: to_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
            funds: vec![],
        })])
    }

    #[cfg(all(not(feature = "token_factory"), not(feature = "osmosis_token_factory")))]
    Ok(vec![CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: liquidity_asset,
        msg: to_binary(&Cw20ExecuteMsg::Mint { recipient, amount })?,
        funds: vec![],
    })])
}

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128, Uint256,
    WasmMsg,
};

use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::traits::AssetReference;
use white_whale::vault_manager::{CallbackMsg, ExecuteMsg};

use crate::state::{CONFIG, ONGOING_FLASHLOAN, VAULTS};
use crate::ContractError;

/// Takes a flashloan of the specified asset and executes the payload.
pub fn flash_loan(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
    payload: Vec<CosmosMsg>,
) -> Result<Response, ContractError> {
    // check that flash loans are enabled
    let config = CONFIG.load(deps.storage)?;
    if !config.flash_loan_enabled {
        return Err(ContractError::Unauthorized {});
    }

    // toggle on the flashloan indicator
    ONGOING_FLASHLOAN.update::<_, StdError>(deps.storage, |_| Ok(true))?;

    // store current balance for after trade profit check
    let old_asset_balance = match asset.info.clone() {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address.clone(), denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let balance_response: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.clone().into_string(),
                },
            )?;
            balance_response.balance
        }
    };

    let mut messages: Vec<CosmosMsg> = vec![];

    // call payload and add after flashloan callback afterwards
    messages.append(&mut payload.clone());
    messages.push(
        WasmMsg::Execute {
            contract_addr: env.contract.address.into_string(),
            msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterFlashloan {
                old_asset_balance,
                loan_asset: asset.clone(),
                sender: info.sender,
            }))?,
            funds: vec![],
        }
        .into(),
    );

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("method", "flash_loan"),
        ("asset", &asset.to_string()),
    ]))
}

/// Processes callback to this contract. Callbacks can only be done by the contract itself.
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
        CallbackMsg::AfterFlashloan {
            old_asset_balance: old_balance,
            loan_asset,
            sender,
        } => after_flashloan(deps, env, old_balance, loan_asset, sender),
    }
}

/// Completes the flashloan by paying back the outstanding loan, fees and returning profits to the
/// original sender.
pub fn after_flashloan(
    deps: DepsMut,
    env: Env,
    old_balance: Uint128,
    loan_asset: Asset,
    sender: Addr,
) -> Result<Response, ContractError> {
    // query new loan asset balance
    let new_asset_balance = match loan_asset.info.clone() {
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

    // calculate the fees for executing the flashloan
    let vault = VAULTS
        .may_load(deps.storage, loan_asset.info.get_reference())?
        .map_or_else(|| Err(ContractError::NonExistentVault {}), Ok)?;

    let protocol_fee = Uint128::try_from(
        vault
            .fees
            .protocol_fee
            .compute(Uint256::from(loan_asset.amount)),
    )?;

    // flashloan fee stays in the vault
    let flash_loan_fee = Uint128::try_from(
        vault
            .fees
            .flash_loan_fee
            .compute(Uint256::from(loan_asset.amount)),
    )?;

    // check that new balance is greater than expected
    let required_amount = old_balance
        .checked_add(protocol_fee)?
        .checked_add(flash_loan_fee)?;

    if required_amount > new_asset_balance {
        return Err(ContractError::NegativeProfit {
            old_balance,
            current_balance: new_asset_balance,
            required_amount,
        });
    }

    // calculate flashloan profit
    let profit = new_asset_balance
        .checked_sub(old_balance)?
        .checked_sub(protocol_fee)?
        .checked_sub(flash_loan_fee)?;

    let mut messages: Vec<CosmosMsg> = vec![];

    if !profit.is_zero() {
        let profit_asset = Asset {
            info: loan_asset.info.clone(),
            amount: profit,
        };

        // send profit to sender
        messages.push(profit_asset.into_msg(sender)?);
    }
    let config = CONFIG.load(deps.storage)?;
    let protocol_fee_asset = Asset {
        info: loan_asset.info.clone(),
        amount: protocol_fee,
    };
    //todo protocol_fee to be sent to "fee collector", i.e. with hook directly to the whale lair. `config.fee_collector_addr` to be remade
    messages.push(protocol_fee_asset.into_msg(config.fee_collector_addr)?);

    // toggle off flashloan indicator
    ONGOING_FLASHLOAN
        .update::<_, StdError>(deps.storage, |_| Ok(false))
        .unwrap();

    Ok(Response::default()
        .add_messages(messages)
        .add_attributes(vec![
            ("action", "after_flashloan".to_string()),
            ("profit", profit.to_string()),
            ("protocol_fee", protocol_fee.to_string()),
            ("flash_loan_fee", flash_loan_fee.to_string()),
        ]))
}

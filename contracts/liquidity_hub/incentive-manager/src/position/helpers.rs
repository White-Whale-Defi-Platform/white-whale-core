use cosmwasm_std::{
    to_json_binary, Addr, Decimal256, Deps, Env, MessageInfo, Order, StdResult, Storage, Uint128,
    WasmMsg,
};
use cw_utils::PaymentError;

use white_whale::incentive_manager::{EpochId, PositionParams};
use white_whale::pool_network::asset::{Asset, AssetInfo};

use crate::state::{ADDRESS_LP_WEIGHT, ADDRESS_LP_WEIGHT_HISTORY, LP_WEIGHTS_HISTORY};
use crate::ContractError;

/// Validates that the message sender has sent the tokens to the contract.
/// In case the `lp_token` is a cw20 token, check if the sender set the specified `amount` as an
/// allowance for us to transfer for `lp_token`.
///
/// If `lp_token` is a native token, check if the funds were sent in the [`MessageInfo`] struct.
///
/// Returns the [`WasmMsg`] that will transfer the specified `amount` of the
/// `lp_token` to the contract.
pub fn validate_funds_sent(
    deps: &Deps,
    env: Env,
    info: MessageInfo,
    lp_asset: Asset,
) -> Result<Option<WasmMsg>, ContractError> {
    if lp_asset.amount.is_zero() {
        return Err(ContractError::PaymentError(PaymentError::NoFunds {}));
    }

    let send_lp_deposit_msg = match lp_asset.info {
        AssetInfo::Token { contract_addr } => {
            let allowance: cw20::AllowanceResponse = deps.querier.query_wasm_smart(
                contract_addr.clone(),
                &cw20::Cw20QueryMsg::Allowance {
                    owner: info.sender.clone().into_string(),
                    spender: env.contract.address.clone().into_string(),
                },
            )?;

            if allowance.allowance < lp_asset.amount {
                return Err(ContractError::MissingPositionDeposit {
                    allowance_amount: allowance.allowance,
                    deposited_amount: lp_asset.amount,
                });
            }

            // send the lp deposit to us
            Some(WasmMsg::Execute {
                contract_addr,
                msg: to_json_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.into_string(),
                    recipient: env.contract.address.into_string(),
                    amount: lp_asset.amount,
                })?,
                funds: vec![],
            })
        }
        AssetInfo::NativeToken { denom } => {
            let paid_amount = cw_utils::must_pay(&info, &denom)?;
            if paid_amount != lp_asset.amount {
                return Err(ContractError::MissingPositionDepositNative {
                    desired_amount: lp_asset.amount,
                    paid_amount,
                });
            }
            // no message needed as native tokens are transferred together with the transaction
            None
        }
    };

    Ok(send_lp_deposit_msg)
}

const SECONDS_IN_DAY: u64 = 86400;
const SECONDS_IN_YEAR: u64 = 31556926;

/// Calculates the weight size for a user filling a position
pub fn calculate_weight(
    lp_asset: &Asset,
    unlocking_duration: u64,
) -> Result<Uint128, ContractError> {
    if !(SECONDS_IN_DAY..=SECONDS_IN_YEAR).contains(&unlocking_duration) {
        return Err(ContractError::InvalidWeight { unlocking_duration });
    }

    // store in Uint128 form for later
    let amount_uint = lp_asset.amount;

    // interpolate between [(86400, 1), (15778463, 5), (31556926, 16)]
    // note that 31556926 is not exactly one 365-day year, but rather one Earth rotation year
    // similarly, 15778463 is not 1/2 a 365-day year, but rather 1/2 a one Earth rotation year

    // first we need to convert into decimals
    let unlocking_duration = Decimal256::from_atomics(unlocking_duration, 0).unwrap();
    let amount = Decimal256::from_atomics(lp_asset.amount, 0).unwrap();

    let unlocking_duration_squared = unlocking_duration.checked_pow(2)?;
    let unlocking_duration_mul =
        unlocking_duration_squared.checked_mul(Decimal256::raw(109498841))?;
    let unlocking_duration_part =
        unlocking_duration_mul.checked_div(Decimal256::raw(7791996353100889432894))?;

    let next_part = unlocking_duration
        .checked_mul(Decimal256::raw(249042009202369))?
        .checked_div(Decimal256::raw(7791996353100889432894))?;

    let final_part = Decimal256::from_ratio(246210981355969u64, 246918738317569u64);

    let weight: Uint128 = amount
        .checked_mul(
            unlocking_duration_part
                .checked_add(next_part)?
                .checked_add(final_part)?,
        )?
        .atomics()
        .checked_div(10u128.pow(18).into())?
        .try_into()?;

    // we must clamp it to max(computed_value, amount) as
    // otherwise we might get a multiplier of 0.999999999999999998 when
    // computing the final_part decimal value, which is over 200 digits.
    Ok(weight.max(amount_uint))
}

/// Gets the latest available weight snapshot recorded for the given address.
pub fn get_latest_address_weight(
    storage: &dyn Storage,
    address: &Addr,
) -> Result<(EpochId, Uint128), ContractError> {
    Ok(ADDRESS_LP_WEIGHT_HISTORY
        .prefix(address)
        .range(storage, None, None, Order::Descending)
        .take(1) // take only one item, the last item. Since it's being sorted in descending order, it's the latest one.
        .collect::<StdResult<(EpochId, Uint128)>>()?)
}

/// Gets the latest available weight snapshot recorded for the given lp.
pub fn get_latest_lp_weight(
    storage: &dyn Storage,
    lp_asset_key: &[u8],
) -> Result<(EpochId, Uint128), ContractError> {
    Ok(LP_WEIGHTS_HISTORY
        .prefix(lp_asset_key)
        .range(storage, None, None, Order::Descending)
        .take(1) // take only one item, the last item. Since it's being sorted in descending order, it's the latest one.
        .collect::<StdResult<(EpochId, Uint128)>>()?)
}

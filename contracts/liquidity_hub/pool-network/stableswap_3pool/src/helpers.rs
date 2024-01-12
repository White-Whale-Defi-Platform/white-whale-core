use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Decimal, Decimal256, Deps, DepsMut, Env, ReplyOn, Response, StdError, StdResult,
    Storage, SubMsg, Uint128, Uint256, WasmMsg,
};
use cw20::MinterResponse;
use cw_storage_plus::Item;

#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use cosmwasm_std::CosmosMsg;
use white_whale::pool_network::asset::{is_factory_token, Asset, AssetInfo, AssetInfoRaw};
#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::MsgCreateDenom;
#[cfg(feature = "injective")]
use white_whale::pool_network::denom_injective::MsgCreateDenom;
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::MsgCreateDenom;
use white_whale::pool_network::querier::query_token_info;
use white_whale::pool_network::token::InstantiateMsg as TokenInstantiateMsg;
use white_whale::pool_network::trio::{InstantiateMsg, PoolFee};

use crate::contract::INSTANTIATE_REPLY_ID;
use crate::error::ContractError;
use crate::stableswap_math::curve::StableSwap;
use crate::state::{LP_SYMBOL, TRIO_INFO};

pub fn compute_swap(
    offer_pool: Uint128,
    ask_pool: Uint128,
    unswapped_pool: Uint128,
    offer_amount: Uint128,
    pool_fees: PoolFee,
    invariant: StableSwap,
) -> StdResult<SwapComputation> {
    let result = invariant
        .swap_to(offer_amount, offer_pool, ask_pool, unswapped_pool)
        .unwrap();

    let return_amount: Uint256 = result.amount_swapped.into();
    let spread_amount = if Uint256::from(offer_amount) > return_amount {
        Uint256::from(offer_amount) - return_amount
    } else {
        return_amount - Uint256::from(offer_amount)
    };
    let swap_fee_amount: Uint256 = pool_fees.swap_fee.compute(return_amount);
    let protocol_fee_amount: Uint256 = pool_fees.protocol_fee.compute(return_amount);
    let burn_fee_amount: Uint256 = pool_fees.burn_fee.compute(return_amount);

    #[cfg(not(feature = "osmosis"))]
    {
        // swap and protocol fee will be absorbed by the pool. Burn fee amount will be burned on a subsequent msg.
        let return_amount: Uint256 =
            return_amount - swap_fee_amount - protocol_fee_amount - burn_fee_amount;

        Ok(SwapComputation {
            return_amount: return_amount.try_into()?,
            spread_amount: spread_amount.try_into()?,
            swap_fee_amount: swap_fee_amount.try_into()?,
            protocol_fee_amount: protocol_fee_amount.try_into()?,
            burn_fee_amount: burn_fee_amount.try_into()?,
        })
    }

    #[cfg(feature = "osmosis")]
    {
        let osmosis_fee_amount: Uint256 = pool_fees.osmosis_fee.compute(return_amount);

        // swap and protocol fee will be absorbed by the pool. Burn fee amount will be burned on a subsequent msg.
        let return_amount: Uint256 =
            return_amount - swap_fee_amount - protocol_fee_amount - osmosis_fee_amount;

        Ok(SwapComputation {
            return_amount: return_amount.try_into()?,
            spread_amount: spread_amount.try_into()?,
            swap_fee_amount: swap_fee_amount.try_into()?,
            protocol_fee_amount: protocol_fee_amount.try_into()?,
            burn_fee_amount: Uint128::zero(),
            osmosis_fee_amount: osmosis_fee_amount.try_into()?,
        })
    }
}

/// Represents the swap computation values
#[cw_serde]
pub struct SwapComputation {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}

pub fn compute_offer_amount(
    offer_pool: Uint128,
    ask_pool: Uint128,
    unswapped_pool: Uint128,
    ask_amount: Uint128,
    pool_fees: PoolFee,
    invariant: StableSwap,
) -> StdResult<OfferAmountComputation> {
    let fees = {
        let base_fees =
            pool_fees.swap_fee.share + pool_fees.protocol_fee.share + pool_fees.burn_fee.share;

        #[cfg(feature = "osmosis")]
        {
            base_fees + pool_fees.osmosis_fee.share
        }

        #[cfg(not(feature = "osmosis"))]
        {
            base_fees
        }
    };

    let one_minus_commission = Decimal::one() - fees;
    let inv_one_minus_commission = Decimal::one() / one_minus_commission;

    let before_commission_deduction = ask_amount * inv_one_minus_commission;

    let offer_amount = invariant
        .reverse_sim(
            before_commission_deduction,
            offer_pool,
            ask_pool,
            unswapped_pool,
        )
        .unwrap();

    let spread_amount = if before_commission_deduction > offer_amount {
        before_commission_deduction - offer_amount
    } else {
        offer_amount - before_commission_deduction
    };

    let swap_fee_amount = pool_fees
        .swap_fee
        .compute(before_commission_deduction.into());
    let protocol_fee_amount = pool_fees
        .protocol_fee
        .compute(before_commission_deduction.into());
    let burn_fee_amount = pool_fees
        .burn_fee
        .compute(before_commission_deduction.into());

    #[cfg(not(feature = "osmosis"))]
    {
        Ok(OfferAmountComputation {
            offer_amount,
            spread_amount,
            swap_fee_amount: swap_fee_amount.try_into()?,
            protocol_fee_amount: protocol_fee_amount.try_into()?,
            burn_fee_amount: burn_fee_amount.try_into()?,
        })
    }

    #[cfg(feature = "osmosis")]
    {
        let osmosis_fee_amount = pool_fees
            .osmosis_fee
            .compute(before_commission_deduction.into());

        Ok(OfferAmountComputation {
            offer_amount,
            spread_amount,
            swap_fee_amount: swap_fee_amount.try_into()?,
            protocol_fee_amount: protocol_fee_amount.try_into()?,
            burn_fee_amount: burn_fee_amount.try_into()?,
            osmosis_fee_amount: osmosis_fee_amount.try_into()?,
        })
    }
}

/// Represents the offer amount computation values
#[cw_serde]
pub struct OfferAmountComputation {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub swap_fee_amount: Uint128,
    pub protocol_fee_amount: Uint128,
    pub burn_fee_amount: Uint128,
    #[cfg(feature = "osmosis")]
    pub osmosis_fee_amount: Uint128,
}
pub fn assert_slippage_tolerance(
    slippage_tolerance: &Option<Decimal>,
    deposits: &[Uint128; 3],
    pools: &[Asset; 3],
    amount: Uint128,
    pool_token_supply: Uint128,
) -> Result<(), ContractError> {
    if let Some(slippage_tolerance) = *slippage_tolerance {
        let slippage_tolerance: Decimal256 = slippage_tolerance.into();
        if slippage_tolerance > Decimal256::one() {
            return Err(StdError::generic_err("slippage_tolerance cannot bigger than 1").into());
        }
        let one_minus_slippage_tolerance = Decimal256::one() - slippage_tolerance;
        let deposits: [Uint256; 3] = [deposits[0].into(), deposits[1].into(), deposits[2].into()];
        let pools: [Uint256; 3] = [
            pools[0].amount.into(),
            pools[1].amount.into(),
            pools[2].amount.into(),
        ];

        let pools_total = pools[0].checked_add(pools[1])?.checked_add(pools[2])?;
        let deposits_total = deposits[0]
            .checked_add(deposits[1])?
            .checked_add(deposits[2])?;

        let pool_ratio = Decimal256::from_ratio(pools_total, pool_token_supply);
        let deposit_ratio = Decimal256::from_ratio(deposits_total, amount);

        if pool_ratio * one_minus_slippage_tolerance > deposit_ratio {
            return Err(ContractError::MaxSlippageAssertion {});
        }
    }

    Ok(())
}

/// Gets the protocol fee amount for the given asset_id
pub fn get_protocol_fee_for_asset(
    collected_protocol_fees: Vec<Asset>,
    asset_id: String,
) -> Uint128 {
    let protocol_fee_asset = collected_protocol_fees
        .iter()
        .find(|&protocol_fee_asset| protocol_fee_asset.clone().get_id() == asset_id.clone())
        .cloned();

    // get the protocol fee for the given pool_asset
    if let Some(protocol_fee_asset) = protocol_fee_asset {
        protocol_fee_asset.amount
    } else {
        Uint128::zero()
    }
}

/// Instantiates fees for a given fee_storage_item
pub fn instantiate_fees(
    storage: &mut dyn Storage,
    asset_info_0: AssetInfo,
    asset_info_1: AssetInfo,
    asset_info_2: AssetInfo,
    fee_storage_item: Item<Vec<Asset>>,
) -> StdResult<()> {
    fee_storage_item.save(
        storage,
        &vec![
            Asset {
                info: asset_info_0,
                amount: Uint128::zero(),
            },
            Asset {
                info: asset_info_1,
                amount: Uint128::zero(),
            },
            Asset {
                info: asset_info_2,
                amount: Uint128::zero(),
            },
        ],
    )
}

/// Gets the total supply of the given liquidity token
pub fn get_total_share(deps: &Deps, liquidity_token: String) -> StdResult<Uint128> {
    #[cfg(any(
        feature = "token_factory",
        feature = "osmosis_token_factory",
        feature = "injective"
    ))]
    let total_share = if is_factory_token(liquidity_token.as_str()) {
        //bank query total
        deps.querier.query_supply(&liquidity_token)?.amount
    } else {
        query_token_info(
            &deps.querier,
            deps.api.addr_validate(liquidity_token.as_str())?,
        )?
        .total_supply
    };
    #[cfg(all(
        not(feature = "token_factory"),
        not(feature = "osmosis_token_factory"),
        not(feature = "injective")
    ))]
    let total_share = query_token_info(
        &deps.querier,
        deps.api.addr_validate(liquidity_token.as_str())?,
    )?
    .total_supply;

    Ok(total_share)
}

/// Verifies if there's a factory token in the vector of [AssetInfo]s.
/// todo consolidate this once the pool PRs are merged
pub fn has_factory_token(assets: &[AssetInfo]) -> bool {
    assets.iter().any(|asset| match asset {
        AssetInfo::Token { .. } => false,
        AssetInfo::NativeToken { denom } => is_factory_token(denom),
    })
}

/// Creates a new LP token for this pool
pub fn create_lp_token(
    deps: DepsMut,
    env: &Env,
    msg: &InstantiateMsg,
    lp_token_name: &String,
) -> Result<Response, ContractError> {
    if msg.token_factory_lp {
        // create native LP token
        TRIO_INFO.update(deps.storage, |mut trio_info| -> StdResult<_> {
            let denom = format!("{}/{}/{}", "factory", env.contract.address, LP_SYMBOL);
            trio_info.liquidity_token = AssetInfoRaw::NativeToken { denom };

            Ok(trio_info)
        })?;

        #[cfg(any(
            feature = "token_factory",
            feature = "osmosis_token_factory",
            feature = "injective"
        ))]
        return Ok(
            Response::new().add_message(<MsgCreateDenom as Into<CosmosMsg>>::into(
                MsgCreateDenom {
                    sender: env.contract.address.to_string(),
                    subdenom: LP_SYMBOL.to_string(),
                },
            )),
        );
        #[allow(unreachable_code)]
        Err(ContractError::TokenFactoryNotEnabled {})
    } else {
        Ok(Response::new().add_submessage(SubMsg {
            // Create LP token
            msg: WasmMsg::Instantiate {
                admin: None,
                code_id: msg.token_code_id,
                msg: to_binary(&TokenInstantiateMsg {
                    name: lp_token_name.to_owned(),
                    symbol: "uLP".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                })?,
                funds: vec![],
                label: lp_token_name.to_owned(),
            }
            .into(),
            gas_limit: None,
            id: INSTANTIATE_REPLY_ID,
            reply_on: ReplyOn::Success,
        }))
    }
}

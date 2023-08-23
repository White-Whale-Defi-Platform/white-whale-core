use classic_bindings::TerraQuery;
use cosmwasm_std::{to_binary, Binary, Decimal, Deps, Env, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg};

use white_whale::pool_network::asset::{get_total_share, AssetInfo};

use crate::error::VaultError;
use crate::state::COLLECTED_PROTOCOL_FEES;
use crate::state::CONFIG;

pub fn get_share(deps: Deps<TerraQuery>, env: Env, amount: Uint128) -> Result<Binary, VaultError> {
    let config = CONFIG.load(deps.storage)?;

    let liquidity_asset = match config.lp_asset.clone() {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { denom } => denom,
    };

    let lp_amount = get_total_share(&deps, liquidity_asset)?;

    let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    let balance = match config.asset_info {
        AssetInfo::NativeToken { denom } => {
            deps.querier
                .query_balance(env.contract.address, denom)?
                .amount
        }
        AssetInfo::Token { contract_addr } => {
            let balance: BalanceResponse = deps.querier.query_wasm_smart(
                contract_addr,
                &Cw20QueryMsg::Balance {
                    address: env.contract.address.into_string(),
                },
            )?;
            balance.balance
        }
    } // deduct protocol fees
    .checked_sub(collected_protocol_fees.amount)?;

    // lp_share = amount / lp_amount
    // asset_share = lp_share * balance
    let asset_share = Decimal::from_ratio(amount, lp_amount) * balance;
    Ok(to_binary(&asset_share)?)
}

// #[cfg(test)]
// #[cfg(not(target_arch = "wasm32"))]
// mod test {
//     use cosmwasm_std::{coins, from_binary, testing::mock_env, Addr, Uint128};
//
//     use white_whale::pool_network::asset::{Asset, AssetInfo};
//     use white_whale::vault_network::vault::Config;
//
//     use crate::state::COLLECTED_PROTOCOL_FEES;
//     use crate::{
//         contract::query,
//         state::CONFIG,
//         tests::{get_fees, mock_creator, mock_dependencies_lp},
//     };
//
//     #[test]
//     fn does_get_share_native() {
//         let env = mock_env();
//         let mut deps = mock_dependencies_lp(
//             &[(
//                 &env.clone().contract.address.into_string(),
//                 &coins(100_000, "uluna"),
//             )],
//             &[
//                 (
//                     mock_creator().sender.into_string(),
//                     &[("lp_token".to_string(), Uint128::new(15_000))],
//                 ),
//                 (
//                     "random_person".to_string(),
//                     &[("lp_token".to_string(), Uint128::new(12_345))],
//                 ),
//             ],
//             vec![],
//         );
//
//         CONFIG
//             .save(
//                 &mut deps.storage,
//                 &Config {
//                     owner: mock_creator().sender,
//                     lp_asset: AssetInfo::Token {
//                         contract_addr: "lp_token".to_string(),
//                     },
//                     asset_info: AssetInfo::NativeToken {
//                         denom: "uluna".to_string(),
//                     },
//                     deposit_enabled: true,
//                     flash_loan_enabled: true,
//                     withdraw_enabled: true,
//                     fee_collector_addr: Addr::unchecked("fee_collector"),
//                     fees: get_fees(),
//                 },
//             )
//             .unwrap();
//
//         // inject protocol fees
//         // half of the liquid supply are fees
//         COLLECTED_PROTOCOL_FEES
//             .save(
//                 &mut deps.storage,
//                 &Asset {
//                     amount: Uint128::new(50_000),
//                     info: AssetInfo::NativeToken {
//                         denom: "uluna".to_string(),
//                     },
//                 },
//             )
//             .unwrap();
//
//         let res: Uint128 = from_binary(
//             &query(
//                 deps.as_ref(),
//                 env,
//                 white_whale::vault_network::vault::QueryMsg::Share {
//                     amount: Uint128::new(15_000),
//                 },
//             )
//             .unwrap(),
//         )
//         .unwrap();
//
//         assert_eq!(res, Uint128::new(27_427));
//     }
//
//     #[test]
//     fn does_get_share_token() {
//         let env = mock_env();
//         let mut deps = mock_dependencies_lp(
//             &[],
//             &[
//                 (
//                     env.clone().contract.address.into_string(),
//                     &[("vault_token".to_string(), Uint128::new(100_000))],
//                 ),
//                 (
//                     mock_creator().sender.into_string(),
//                     &[("lp_token".to_string(), Uint128::new(15_000))],
//                 ),
//                 (
//                     "random_person".to_string(),
//                     &[("lp_token".to_string(), Uint128::new(12_345))],
//                 ),
//             ],
//             vec![],
//         );
//
//         CONFIG
//             .save(
//                 &mut deps.storage,
//                 &Config {
//                     owner: mock_creator().sender,
//                     lp_asset: AssetInfo::Token {
//                         contract_addr: "lp_token".to_string(),
//                     },
//                     asset_info: AssetInfo::Token {
//                         contract_addr: "vault_token".to_string(),
//                     },
//                     deposit_enabled: true,
//                     flash_loan_enabled: true,
//                     withdraw_enabled: true,
//                     fee_collector_addr: Addr::unchecked("fee_collector"),
//                     fees: get_fees(),
//                 },
//             )
//             .unwrap();
//
//         // inject protocol fees
//         // half of the liquid supply are fees
//         COLLECTED_PROTOCOL_FEES
//             .save(
//                 &mut deps.storage,
//                 &Asset {
//                     amount: Uint128::new(50_000),
//                     info: AssetInfo::Token {
//                         contract_addr: "vault_token".to_string(),
//                     },
//                 },
//             )
//             .unwrap();
//
//         let res: Uint128 = from_binary(
//             &query(
//                 deps.as_ref(),
//                 env,
//                 white_whale::vault_network::vault::QueryMsg::Share {
//                     amount: Uint128::new(15_000),
//                 },
//             )
//             .unwrap(),
//         )
//         .unwrap();
//
//         assert_eq!(res, Uint128::new(27_427));
//     }
// }

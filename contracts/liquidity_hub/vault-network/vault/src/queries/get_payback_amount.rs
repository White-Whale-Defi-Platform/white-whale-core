use classic_bindings::TerraQuery;
use cosmwasm_std::{to_binary, Binary, Deps, Uint128, Uint256};
use white_whale::pool_network::asset::{Asset, AssetInfo};
use white_whale::vault_network::vault::PaybackAmountResponse;

use crate::error::VaultError;
use crate::state::CONFIG;

pub fn get_payback_amount(deps: Deps<TerraQuery>, amount: Uint128) -> Result<Binary, VaultError> {
    let config = CONFIG.load(deps.storage)?;

    // check that balance is greater than expected
    let protocol_fee = Uint128::try_from(config.fees.protocol_fee.compute(Uint256::from(amount)))?;
    let flash_loan_fee =
        Uint128::try_from(config.fees.flash_loan_fee.compute(Uint256::from(amount)))?;
    let burn_fee = Uint128::try_from(config.fees.burn_fee.compute(Uint256::from(amount)))?;

    let payback_asset = Asset {
        info: config.asset_info.clone(),
        amount,
    };

    let tax = payback_asset.compute_tax(&deps.querier)?;

    let required_amount = amount
        .checked_add(protocol_fee)?
        .checked_add(flash_loan_fee)?
        .checked_add(burn_fee)?
        .checked_add(tax)?;

    Ok(to_binary(&PaybackAmountResponse {
        payback_amount: required_amount,
        protocol_fee,
        flash_loan_fee,
        burn_fee,
    })?)
}

// #[cfg(test)]
// #[cfg(not(target_arch = "wasm32"))]
// mod test {
//     use crate::contract::query;
//     use crate::state::CONFIG;
//     use crate::tests::mock_creator;
//     use cosmwasm_std::testing::{mock_dependencies, mock_env};
//     use cosmwasm_std::{from_binary, Addr, Decimal, Uint128};
//     use white_whale::fee::{Fee, VaultFee};
//     use white_whale::pool_network::asset::AssetInfo;
//     use white_whale::vault_network::vault::{Config, PaybackAmountResponse, QueryMsg};
//
//     #[test]
//     fn returns_payback_amount() {
//         let mut deps = mock_dependencies();
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
//                     fees: VaultFee {
//                         flash_loan_fee: Fee {
//                             share: Decimal::permille(5),
//                         },
//                         protocol_fee: Fee {
//                             share: Decimal::permille(5),
//                         },
//                         burn_fee: Fee {
//                             share: Decimal::permille(1),
//                         },
//                     },
//                 },
//             )
//             .unwrap();
//
//         let res: PaybackAmountResponse = from_binary(
//             &query(
//                 deps.as_ref(),
//                 mock_env(),
//                 QueryMsg::GetPaybackAmount {
//                     amount: Uint128::new(1000),
//                 },
//             )
//             .unwrap(),
//         )
//         .unwrap();
//
//         assert_eq!(
//             res,
//             PaybackAmountResponse {
//                 payback_amount: Uint128::new(1011),
//                 protocol_fee: Uint128::new(5),
//                 flash_loan_fee: Uint128::new(5),
//                 burn_fee: Uint128::new(1),
//             }
//         );
//     }
// }

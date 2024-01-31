use classic_bindings::TerraQuery;
use cosmwasm_std::{DepsMut, Response};

use white_whale_std::pool_network::asset::AssetInfo;

use crate::asset::AssetReference;
use crate::err::{StdResult, VaultFactoryError};
use crate::state::VAULTS;

pub fn remove_vault(deps: DepsMut<TerraQuery>, asset_info: AssetInfo) -> StdResult<Response> {
    if let Ok(None) = VAULTS.may_load(deps.storage, asset_info.get_reference()) {
        return Err(VaultFactoryError::NonExistentVault {});
    }

    VAULTS.remove(deps.storage, asset_info.get_reference());

    Ok(Response::new().add_attributes(vec![("method", "remove_vault")]))
}

// #[cfg(test)]
// #[cfg(not(target_arch = "wasm32"))]
// mod tests {
//     use cosmwasm_std::{testing::mock_info, Attribute};
//     use cw_multi_test::Executor;
//
//     use crate::{
//         contract::execute,
//         err::VaultFactoryError,
//         tests::{
//             get_fees, mock_app, mock_creator,
//             mock_instantiate::{app_mock_instantiate, mock_instantiate},
//         },
//     };
//
//     #[test]
//     fn can_remove_vault() {
//         let mut app = mock_app();
//         let creator = mock_creator();
//
//         let factory_addr = app_mock_instantiate(&mut app);
//
//         // create vault
//         let asset_info_1 = white_whale::pool_network::asset::AssetInfo::NativeToken {
//             denom: "uluna".to_string(),
//         };
//
//         app.execute_contract(
//             creator.sender.clone(),
//             factory_addr.clone(),
//             &white_whale::vault_network::vault_factory::ExecuteMsg::CreateVault {
//                 asset_info: asset_info_1.clone(),
//                 fees: get_fees(),
//                 token_factory_lp: false,
//             },
//             &[],
//         )
//         .unwrap();
//
//         // remove vault
//         let res = app
//             .execute_contract(
//                 creator.sender,
//                 factory_addr.clone(),
//                 &white_whale::vault_network::vault_factory::ExecuteMsg::RemoveVault {
//                     asset_info: asset_info_1,
//                 },
//                 &[],
//             )
//             .unwrap();
//
//         assert_eq!(res.events.len(), 2);
//
//         for event in res.events {
//             if event.ty == "wasm" {
//                 let expected_attributes = vec![
//                     Attribute {
//                         key: "_contract_addr".to_string(),
//                         value: factory_addr.clone().to_string(),
//                     },
//                     Attribute {
//                         key: "method".to_string(),
//                         value: "remove_vault".to_string(),
//                     },
//                 ];
//
//                 assert_eq!(event.attributes, expected_attributes);
//             }
//         }
//     }
//
//     #[test]
//     fn cannot_remove_vault_unauthorized() {
//         let asset_info = white_whale::pool_network::asset::AssetInfo::NativeToken {
//             denom: "uluna".to_string(),
//         };
//         let (mut deps, env) = mock_instantiate(5, 6);
//
//         // migrate a vault unauthorized
//         let bad_actor = mock_info("not_owner", &[]);
//
//         let res = execute(
//             deps.as_mut(),
//             env,
//             bad_actor,
//             white_whale::vault_network::vault_factory::ExecuteMsg::RemoveVault { asset_info },
//         );
//
//         assert_eq!(res.unwrap_err(), VaultFactoryError::Unauthorized {})
//     }
//
//     #[test]
//     fn cannot_remove_vault_non_existent() {
//         let asset_info = white_whale::pool_network::asset::AssetInfo::NativeToken {
//             denom: "uluna".to_string(),
//         };
//         let (mut deps, env) = mock_instantiate(5, 6);
//         let creator = mock_creator();
//
//         // remove non-existent vault
//         let res = execute(
//             deps.as_mut(),
//             env,
//             creator,
//             white_whale::vault_network::vault_factory::ExecuteMsg::RemoveVault { asset_info },
//         );
//
//         assert_eq!(res.unwrap_err(), VaultFactoryError::NonExistentVault {})
//     }
// }

use classic_bindings::TerraQuery;
use cosmwasm_std::{to_json_binary, Binary, Deps};

use crate::{err::StdResult, state::CONFIG};

/// Retrieves the contract configuration stored in state.
pub fn get_config(deps: Deps<TerraQuery>) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;

    Ok(to_json_binary(&config)?)
}

// #[cfg(test)]
// #[cfg(not(target_arch = "wasm32"))]
// mod test {
//     use cosmwasm_std::Addr;
//     use white_whale::vault_network::vault_router::{Config, QueryMsg};
//
//     use crate::tests::{mock_creator, mock_query};
//
//     #[test]
//     fn does_get_config() {
//         let (config, ..) = mock_query::<Config>("factory_addr".to_string(), QueryMsg::Config {});
//
//         assert_eq!(
//             config,
//             Config {
//                 owner: mock_creator().sender,
//                 vault_factory: Addr::unchecked("factory_addr"),
//             }
//         );
//     }
// }

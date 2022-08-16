use cosmwasm_std::{to_binary, Binary, Deps};
use terraswap::asset::AssetInfo;

use crate::{asset::AssetReference, err::StdResult, state::VAULTS};

pub fn get_vault(deps: Deps, asset_info: AssetInfo) -> StdResult<Binary> {
    Ok(to_binary(
        &VAULTS.may_load(deps.storage, asset_info.get_reference())?,
    )?)
}

#[cfg(test)]
mod tests {
    use cw_multi_test::Executor;
    use vault_network::vault_factory::{ExecuteMsg, QueryMsg};

    use crate::tests::{
        get_fees, mock_app, mock_creator, mock_instantiate::app_mock_instantiate, mock_query,
    };

    #[test]
    fn does_return_none_for_no_vault() {
        let (res, ..) = mock_query::<Option<String>>(
            5,
            6,
            QueryMsg::Vault {
                asset_info: terraswap::asset::AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
        );

        assert_eq!(res, None);
    }

    #[test]
    fn does_get_created_vault_address() {
        let mut app = mock_app();
        let factory_addr = app_mock_instantiate(&mut app);

        let creator = mock_creator();

        // create a vault
        let asset_info = terraswap::asset::AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        let res = app
            .execute_contract(
                creator.sender,
                factory_addr.clone(),
                &ExecuteMsg::CreateVault {
                    asset_info: asset_info.clone(),
                    fees: get_fees(),
                },
                &[],
            )
            .unwrap();

        let created_vault_addr = res
            .events
            .iter()
            .flat_map(|event| &event.attributes)
            .find(|attribute| attribute.key == "vault_address")
            .unwrap();

        // check that the address was stored
        let vault_addr: Option<String> = app
            .wrap()
            .query_wasm_smart(factory_addr, &QueryMsg::Vault { asset_info })
            .unwrap();

        assert_eq!(vault_addr, Some(created_vault_addr.value.clone()));
    }
}

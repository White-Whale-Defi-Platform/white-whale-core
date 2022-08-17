use cosmwasm_std::{to_binary, Binary, Deps};

use terraswap::asset::AssetInfo;
use vault_network::vault_factory::{VaultInfo, VaultsResponse};

use crate::state::read_vaults;
use crate::{asset::AssetReference, err::StdResult, state::VAULTS};

pub fn get_vault(deps: Deps, asset_info: AssetInfo) -> StdResult<Binary> {
    Ok(to_binary(
        &VAULTS.may_load(deps.storage, asset_info.get_reference())?,
    )?)
}

pub fn get_vaults(
    deps: Deps,
    start_after: Option<Vec<u8>>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let vaults: Vec<VaultInfo> = read_vaults(deps.storage, deps.api, start_after, limit)?;
    Ok(to_binary(&VaultsResponse { vaults })?)
}

#[cfg(test)]
mod tests {
    use cw_multi_test::Executor;

    use vault_network::vault_factory::{ExecuteMsg, QueryMsg, VaultsResponse};

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

    #[test]
    fn does_get_vault_addresses() {
        let mut app = mock_app();
        let factory_addr = app_mock_instantiate(&mut app);

        let creator = mock_creator();

        // create a vault

        let alphabet = vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];
        let mut vault_addresses: Vec<String> = Vec::new();
        for i in 0..7 {
            let asset_info = terraswap::asset::AssetInfo::NativeToken {
                denom: format!("uluna{}", alphabet[i]),
            };

            let res = app
                .execute_contract(
                    creator.sender.clone(),
                    factory_addr.clone(),
                    &ExecuteMsg::CreateVault {
                        asset_info: asset_info.clone(),
                        fees: get_fees().clone(),
                    },
                    &[],
                )
                .unwrap();
            println!("here");
            let created_vault_addr = res
                .events
                .iter()
                .flat_map(|event| &event.attributes)
                .find(|attribute| attribute.key == "vault_address")
                .unwrap();

            println!("created_vault_addr:: {}", created_vault_addr.value);

            vault_addresses.push(created_vault_addr.value.clone());
        }

        // check that the addresses were stored, without pagination. Default limit is 10, so it
        // will return all vaults with a single call
        let vaults_response: VaultsResponse = app
            .wrap()
            .query_wasm_smart(
                factory_addr.clone(),
                &QueryMsg::Vaults {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        assert_eq!(vaults_response.vaults.len(), 7usize);
        assert_eq!(vaults_response.vaults[0].vault, vault_addresses[0]);
        assert_eq!(vaults_response.vaults[5].vault, vault_addresses[5]);

        // check that the addresses were stored, with pagination
        let mut paginated_vault_addresses: Vec<String> = Vec::new();
        let mut start_after: Option<Vec<u8>> = None;
        let mut i = 0;
        // there are 7 vaults in the factory, let's take 4 vaults at a time so we query 2 times
        while i < 2 {
            let vaults_response: VaultsResponse = app
                .wrap()
                .query_wasm_smart(
                    factory_addr.clone(),
                    &QueryMsg::Vaults {
                        start_after: start_after.clone(),
                        limit: Some(u32::try_from(4).unwrap()),
                    },
                )
                .unwrap();

            start_after = Some(
                vaults_response
                    .clone()
                    .vaults
                    .last()
                    .unwrap()
                    .asset_info_reference
                    .clone(),
            );

            let vaults: Vec<String> = vaults_response
                .vaults
                .iter()
                .map(|vault_info| vault_info.vault.clone())
                .collect();

            paginated_vault_addresses = paginated_vault_addresses
                .into_iter()
                .chain(vaults.into_iter())
                .collect();
            i += 1;
        }

        assert_eq!(paginated_vault_addresses.len(), 7usize);
        assert_eq!(paginated_vault_addresses[0], vault_addresses[0]);
        assert_eq!(paginated_vault_addresses[5], vault_addresses[5]);
    }
}

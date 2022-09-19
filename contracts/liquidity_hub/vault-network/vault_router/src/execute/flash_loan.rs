use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, MessageInfo, Response, WasmMsg};
use terraswap::asset::Asset;
use vault_network::vault_router::ExecuteMsg;

use crate::{
    err::{StdResult, VaultRouterError},
    state::CONFIG,
};

pub fn flash_loan(
    deps: DepsMut,
    info: MessageInfo,
    assets: Vec<Asset>,
    msgs: Vec<CosmosMsg>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    // get the vaults to perform loans for
    let vaults = assets
        .into_iter()
        .map(|asset| {
            // query factory for address
            let address: Option<String> = deps.querier.query_wasm_smart(
                config.vault_factory.clone(),
                &vault_network::vault_factory::QueryMsg::Vault {
                    asset_info: asset.info.clone(),
                },
            )?;

            // return InvalidAsset if address doesn't exist
            let address = address.ok_or(VaultRouterError::InvalidAsset {
                asset: asset.clone(),
            })?;

            Ok((address, asset))
        })
        .collect::<StdResult<Vec<_>>>()?;

    let mut messages: Vec<CosmosMsg> = vec![];

    // run all the loans
    if let Some(((vault, asset), next_vaults)) = vaults.split_first() {
        messages.push(
            WasmMsg::Execute {
                contract_addr: vault.to_string(),
                msg: to_binary(&vault_network::vault::ExecuteMsg::FlashLoan {
                    amount: asset.amount,
                    msg: to_binary(&ExecuteMsg::NextLoan {
                        initiator: info.sender,
                        to_loan: next_vaults.to_vec(),
                        payload: msgs,
                        loaned_assets: vaults,
                    })?,
                })?,
                funds: vec![],
            }
            .into(),
        );
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(vec![("method", "flash_loan")]))
}

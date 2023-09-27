use cosmwasm_std::{to_binary, CosmosMsg, WasmMsg};

use white_whale::pool_network::asset::{Asset, ToCoins};
use white_whale::whale_lair;

use crate::ContractError;

/// Creates a message to fill rewards on the whale lair contract.
pub(crate) fn fill_rewards_msg(
    contract_addr: String,
    assets: Vec<Asset>,
) -> Result<CosmosMsg, ContractError> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr,
        msg: to_binary(&whale_lair::ExecuteMsg::FillRewards {
            assets: assets.clone(),
        })?,
        funds: assets.to_coins()?,
    }))
}

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Binary, CosmosMsg, StdResult, WasmMsg};

use crate::epoch_manager::epoch_manager::EpochV2;

#[cw_serde]
pub struct EpochChangedHookMsg {
    pub current_epoch: EpochV2,
}

impl EpochChangedHookMsg {
    /// serializes the message
    pub fn into_json_binary(self) -> StdResult<Binary> {
        let msg = EpochChangedExecuteMsg::EpochChangedHook(self);
        to_json_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>>(self, contract_addr: T) -> StdResult<CosmosMsg> {
        let msg = self.into_json_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

#[cw_serde]
enum EpochChangedExecuteMsg {
    EpochChangedHook(EpochChangedHookMsg),
}

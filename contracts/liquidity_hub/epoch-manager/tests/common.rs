use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{DepsMut, MessageInfo, Response, Uint64};

use epoch_manager::contract::{execute, instantiate};
use epoch_manager::ContractError;
use white_whale_std::epoch_manager::epoch_manager::{EpochV2, EpochConfig, ExecuteMsg, InstantiateMsg};

/// Mocks contract instantiation.
pub(crate) fn mock_instantiation(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let current_time = mock_env().block.time;
    let msg = InstantiateMsg {
        start_epoch: EpochV2 {
            id: 123,
            start_time: current_time,
        },
        epoch_config: EpochConfig {
            duration: Uint64::new(86400),
            genesis_epoch: Uint64::new(current_time.nanos()),
        },
    };

    instantiate(deps, mock_env(), info, msg)
}

/// Mocks hook addition.
pub(crate) fn mock_add_hook(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::AddHook {
        contract_addr: "hook_contract_1".to_string(),
    };

    execute(deps, mock_env(), info, msg)
}

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Reply, Response, SubMsg, WasmMsg};
use cw2::{get_contract_version, set_contract_version};
use white_whale::pool_network::frontend_helper::{
    Config, ExecuteMsg, InstantiateMsg, MigrateMsg, TempState,
};

use semver::Version;

use crate::error::ContractError;
use crate::error::ContractError::MigrateInvalidVersion;
use crate::reply;
use crate::reply::deposit_pair::DEPOSIT_PAIR_REPLY_ID;
use crate::state::{CONFIG, TEMP_STATE};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-frontend-helper";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        incentive_factory_addr: deps.api.addr_canonicalize(&msg.incentive_factory)?,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {
            pair_address,
            assets,
            slippage_tolerance,
            unbonding_duration,
        } => {
            TEMP_STATE.save(
                deps.storage,
                &TempState {
                    unbonding_duration,
                    receiver: deps.api.addr_canonicalize(&info.sender.into_string())?,
                },
            )?;

            // send request to deposit
            Ok(Response::new().add_submessage(SubMsg {
                id: DEPOSIT_PAIR_REPLY_ID,
                reply_on: cosmwasm_std::ReplyOn::Always,
                gas_limit: None,
                msg: WasmMsg::Execute {
                    contract_addr: pair_address,
                    msg: to_binary(
                        &white_whale::pool_network::pair::ExecuteMsg::ProvideLiquidity {
                            assets,
                            slippage_tolerance,
                            receiver: None,
                        },
                    )?,
                    funds: info.funds,
                }
                .into(),
            }))
        }
    }
}

/// Handles reply messages from submessages sent out by the frontend contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        DEPOSIT_PAIR_REPLY_ID => reply::deposit_pair::deposit_pair(deps, env, msg),
        id => Err(ContractError::UnknownReplyId { id }),
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

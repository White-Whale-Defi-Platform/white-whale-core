use crate::state::CONFIG;
use crate::ContractError;
use cosmwasm_std::Deps;
use white_whale_std::incentive_manager::Config;

/// Queries the manager config
pub(crate) fn query_manager_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

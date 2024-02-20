use crate::state::INCENTIVES;
use crate::ContractError;
use cosmwasm_std::{Deps, Order, StdResult};
use white_whale::incentive_manager::Incentive;
use white_whale::pool_network::incentive::Flow;

/// Gets the incentives that are available for the current epoch, i.e. those flows that started either on
/// the epoch provided or before it.
pub fn get_claimable_incentives(deps: Deps, epoch: &u64) -> Result<Vec<Incentive>, ContractError> {
    Ok(INCENTIVES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
        .into_iter()
        .filter(|((start_epoch, _), _)| start_epoch <= epoch)
        .map(|(_, flow)| flow)
        .collect::<Vec<Flow>>())
}

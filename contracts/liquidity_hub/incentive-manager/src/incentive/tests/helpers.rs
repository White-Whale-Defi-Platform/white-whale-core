use crate::state::ADDRESS_LP_WEIGHT_HISTORY;
use cosmwasm_std::{Addr, StdResult, Storage, Uint128};

pub(crate) fn fill_address_lp_weight_history(
    storage: &mut dyn Storage,
    address: &Addr,
    epoch_id: u64,
    weight: Uint128,
) -> StdResult<()> {
    ADDRESS_LP_WEIGHT_HISTORY.save(storage, (address, epoch_id), &weight)
}

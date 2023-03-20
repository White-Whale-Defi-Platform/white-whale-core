use cosmwasm_std::{Addr, Deps, StdError};

use crate::state::INCENTIVE_MAPPINGS;

pub fn get_incentive(deps: Deps, lp_address: String) -> Result<Option<Addr>, StdError> {
    let lp_address = deps.api.addr_validate(&lp_address)?;

    INCENTIVE_MAPPINGS.may_load(deps.storage, lp_address)
}

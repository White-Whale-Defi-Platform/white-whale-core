use cosmwasm_std::{Addr, Deps, StdError};
use white_whale::pool_network::asset::AssetInfo;

use crate::state::INCENTIVE_MAPPINGS;

pub fn get_incentive(deps: Deps, lp_address: AssetInfo) -> Result<Option<Addr>, StdError> {
    INCENTIVE_MAPPINGS.may_load(deps.storage, lp_address.to_raw(deps.api)?.as_bytes())
}

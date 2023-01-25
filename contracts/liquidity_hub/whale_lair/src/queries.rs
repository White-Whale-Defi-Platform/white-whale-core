use crate::state::CONFIG;
use cosmwasm_std::{Deps, StdResult};
use white_whale::whale_lair::Config;

pub(crate) fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

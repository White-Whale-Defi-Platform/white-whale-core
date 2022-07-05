use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use terraswap::asset::PairInfoRaw;
use terraswap::pair::{FeatureToggle, PoolFee};

pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");
pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub pool_fees: PoolFee,
    pub feature_toggle: FeatureToggle,
}

pub type ConfigResponse = Config;

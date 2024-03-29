use cw_controllers::{Admin, Hooks};
use cw_storage_plus::Item;
use white_whale_std::epoch_manager::epoch_manager::{Config, EpochV2};

pub const CONFIG: Item<Config> = Item::new("config");
pub const ADMIN: Admin = Admin::new("admin");
pub const HOOKS: Hooks = Hooks::new("hooks");
pub const EPOCH: Item<EpochV2> = Item::new("epoch");

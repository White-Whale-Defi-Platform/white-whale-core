use cw_controllers::{Admin, Hooks};
use cw_storage_plus::{Item, Map};
use white_whale::epoch_manager::epoch_manager::{Config, Epoch};

pub const CONFIG: Item<Config> = Item::new("config");
pub const ADMIN: Admin = Admin::new("admin");
pub const HOOKS: Hooks = Hooks::new("hooks");
pub const EPOCHS: Map<&[u8], Epoch> = Map::new("epochs");

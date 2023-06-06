use cw_storage_plus::Item;
use white_whale::pool_network::frontend_helper::{Config, TempState};

pub const CONFIG: Item<Config> = Item::new("config");

pub const TEMP_STATE: Item<TempState> = Item::new("temp_state");

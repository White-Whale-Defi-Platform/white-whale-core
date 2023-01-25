use cw_storage_plus::Item;
use white_whale::whale_lair::Config;

pub const CONFIG: Item<Config> = Item::new("config");

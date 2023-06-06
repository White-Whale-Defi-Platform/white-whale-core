use cw_storage_plus::Item;
use white_whale::fee_distributor::Epoch;

pub const CURRENT_EPOCH: Item<Epoch> = Item::new("epoch");

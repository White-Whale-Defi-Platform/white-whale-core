#[allow(unused_imports)]
use crate::tokenfactory::common::{create_msg, MsgTypes};
use crate::tokenfactory::mint::MsgMint;
use cosmwasm_std::{Addr, Coin, CosmosMsg};

/// Returns the MsgBurn Stargate message
pub fn burn(sender: Addr, coin: Coin) -> CosmosMsg {
    let message_data = MsgMint { coin };
    create_msg(sender, message_data, MsgTypes::Burn.as_str())
}

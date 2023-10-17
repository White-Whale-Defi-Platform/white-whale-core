use crate::tokenfactory::common::{create_msg, MsgTypes};
use crate::tokenfactory::mint::MsgMint;
use cosmwasm_std::{Addr, Coin, CosmosMsg};

/// Returns the MsgBurn Stargate message
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
pub fn burn(sender: Addr, coin: Coin) -> CosmosMsg {
    let message_data = MsgMint { coin };
    create_msg(sender, message_data, MsgTypes::MsgBurn.as_str())
}

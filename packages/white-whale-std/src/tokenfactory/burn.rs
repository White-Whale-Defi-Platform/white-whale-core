#[allow(unused_imports)]
use crate::tokenfactory::common::{create_msg, MsgTypes};
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use crate::tokenfactory::mint::MsgMint;
#[allow(unused_imports)]
use cosmwasm_std::{Addr, Coin, CosmosMsg};

/// Returns the MsgBurn Stargate message
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
pub fn burn(sender: Addr, coin: Coin) -> CosmosMsg {
    let message_data = MsgMint { coin };
    create_msg(sender, message_data, MsgTypes::Burn.as_str())
}

use crate::tokenfactory::common::EncodeMessage;
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use crate::tokenfactory::common::{create_msg, MsgTypes};
use anybuf::Anybuf;
use cosmwasm_std::Coin;
/// Returns the MsgMint Stargate message
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use cosmwasm_std::{Addr, CosmosMsg};

/// Returns the MsgMint Stargate message
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
pub fn mint(sender: Addr, coin: Coin) -> CosmosMsg {
    let message_data = MsgMint { coin };
    create_msg(sender, message_data, MsgTypes::Mint.as_str())
}

pub(crate) struct MsgMint {
    pub coin: Coin,
}

impl EncodeMessage for MsgMint {
    fn encode(sender: String, data: Self) -> Vec<u8> {
        let coin_buf = Anybuf::new()
            .append_string(1, data.coin.denom)
            .append_string(2, data.coin.amount.to_string());

        Anybuf::new()
            .append_string(1, sender)
            .append_message(2, &coin_buf)
            .into_vec()
    }
}

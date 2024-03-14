use std::str::FromStr;

use anybuf::{Anybuf, Bufany};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, CosmosMsg, StdResult, Uint128};

use crate::tokenfactory::common::EncodeMessage;
use crate::tokenfactory::common::{create_msg, MsgTypes};

/// Returns the MsgMint Stargate message
pub fn mint(sender: Addr, coin: Coin, mint_to_address: String) -> CosmosMsg {
    let message_data = MsgMint {
        sender: sender.to_string(),
        amount: coin,
        mint_to_address,
    };

    create_msg(message_data, MsgTypes::Mint.as_str())
}

#[cw_serde]
pub struct MsgMint {
    pub sender: String,
    pub amount: Coin,
    pub mint_to_address: String,
}

impl EncodeMessage for MsgMint {
    fn encode(data: Self) -> Vec<u8> {
        let coin_buf = Anybuf::new()
            .append_string(1, data.amount.denom)
            .append_string(2, data.amount.amount.to_string());

        Anybuf::new()
            .append_string(1, data.sender)
            .append_message(2, &coin_buf)
            .append_string(3, &data.mint_to_address)
            .into_vec()
    }

    fn decode(data: Vec<u8>) -> StdResult<Self>
    where
        Self: Sized,
    {
        let deserialized = Bufany::deserialize(&data).unwrap();

        let coin_msg = deserialized.message(2).unwrap();
        let coin = Coin {
            denom: coin_msg.string(1).unwrap(),
            amount: Uint128::from_str(coin_msg.string(2).unwrap().as_str()).unwrap(),
        };

        Ok(Self {
            sender: deserialized.string(1).unwrap(),
            amount: coin,
            mint_to_address: deserialized.string(3).unwrap(),
        })
    }
}

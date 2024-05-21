use std::str::FromStr;

use crate::tokenfactory::common::{create_msg, MsgTypes};
use anybuf::{Anybuf, Bufany};
use cosmwasm_schema::cw_serde;
#[allow(unused_imports)]
use cosmwasm_std::{Addr, Coin, CosmosMsg};
use cosmwasm_std::{StdResult, Uint128};

use super::common::EncodeMessage;

/// Returns the MsgBurn Stargate message
pub fn burn(sender: Addr, coin: Coin, burn_from_address: String) -> CosmosMsg {
    let message_data = MsgBurn {
        sender: sender.to_string(),
        amount: coin,
        burn_from_address,
    };
    create_msg(message_data, MsgTypes::Burn.as_str())
}

#[cw_serde]
pub struct MsgBurn {
    pub sender: String,
    pub amount: Coin,
    pub burn_from_address: String,
}

impl EncodeMessage for MsgBurn {
    fn encode(data: Self) -> Vec<u8> {
        let coin_buf = Anybuf::new()
            .append_string(1, data.amount.denom)
            .append_string(2, data.amount.amount.to_string());

        Anybuf::new()
            .append_string(1, data.sender)
            .append_message(2, &coin_buf)
            .append_string(3, &data.burn_from_address)
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
            burn_from_address: deserialized.string(3).unwrap(),
        })
    }
}

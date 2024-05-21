use crate::tokenfactory::common::{create_msg, MsgTypes};
use anybuf::{Anybuf, Bufany};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::StdResult;

use cosmwasm_std::{Addr, CosmosMsg};

use crate::tokenfactory::common::EncodeMessage;

pub fn create_denom(sender: Addr, subdenom: String) -> CosmosMsg {
    let message_data = MsgCreateDenom {
        sender: sender.to_string(),
        subdenom,
    };
    create_msg(message_data, MsgTypes::CreateDenom.as_str())
}

#[cw_serde]
pub struct MsgCreateDenom {
    pub sender: String,
    pub subdenom: String,
}

impl EncodeMessage for MsgCreateDenom {
    fn encode(data: Self) -> Vec<u8> {
        Anybuf::new()
            .append_string(1, data.sender)
            .append_string(2, data.subdenom)
            .into_vec()
    }

    fn decode(data: Vec<u8>) -> StdResult<Self>
    where
        Self: Sized,
    {
        let deserialized = Bufany::deserialize(&data).unwrap();
        Ok(Self {
            sender: deserialized.string(1).unwrap(),
            subdenom: deserialized.string(2).unwrap(),
        })
    }
}

/// MsgCreateDenomResponse is the return value of MsgCreateDenom It returns the full string of the newly created denom
#[cw_serde]
pub struct MsgCreateDenomResponse {
    pub new_token_denom: String,
}

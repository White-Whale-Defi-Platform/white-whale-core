use anybuf::Anybuf;
use cosmwasm_std::{Addr, CosmosMsg};

use crate::tokenfactory::common::{create_msg, EncodeMessage, MsgTypes};

/// Returns the MsgCreateDenom Stargate message
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
pub fn create_denom(sender: Addr, subdenom: String) -> CosmosMsg {
    let message_data = MsgCreateDenom { subdenom };
    create_msg(sender, message_data, MsgTypes::MsgCreateDenom.as_str())
}

struct MsgCreateDenom {
    pub subdenom: String,
}

impl EncodeMessage for MsgCreateDenom {
    fn encode(sender: String, data: Self) -> Vec<u8> {
        Anybuf::new()
            .append_string(1, sender)
            .append_string(2, data.subdenom)
            .into_vec()
    }
}

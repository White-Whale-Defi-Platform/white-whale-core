use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg};
#[cw_serde]
enum Protocol {
    Injective,
    CosmWasm,
    Osmosis,
}

impl Protocol {
    fn from_features() -> Self {
        #[cfg(feature = "injective")]
        {
            return Self::Injective;
        }
        #[cfg(feature = "token_factory")]
        {
            return Self::CosmWasm;
        }
        #[cfg(feature = "osmosis_token_factory")]
        {
            return Self::Osmosis;
        }
        unreachable!()
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Injective => "injective",
            Self::CosmWasm => "cosmwasm",
            Self::Osmosis => "osmosis",
        }
    }
}

pub(crate) enum MsgTypes {
    MsgCreateDenom,
    MsgMint,
    MsgBurn,
}

impl MsgTypes {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MsgCreateDenom => "MsgCreateDenom",
            Self::MsgMint => "MsgMint",
            Self::MsgBurn => "MsgBurn",
        }
    }
}

pub(crate) trait EncodeMessage {
    fn encode(sender: String, data: Self) -> Vec<u8>;
}

pub(crate) fn create_msg<M: EncodeMessage>(
    sender: Addr,
    message_data: M,
    msg_type: &str,
) -> CosmosMsg {
    CosmosMsg::Stargate {
        type_url: format!(
            "/{}.tokenfactory.v1beta1.{}",
            Protocol::from_features().as_str(),
            msg_type
        ),
        value: M::encode(sender.into(), message_data).into(),
    }
}

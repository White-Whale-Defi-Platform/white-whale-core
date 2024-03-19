#[cfg(feature = "token_factory")]
use cosmwasm_schema::cw_serde;
#[cfg(feature = "token_factory")]
use cosmwasm_std::{Addr, CosmosMsg};

#[cfg(feature = "token_factory")]
#[cw_serde]
enum Protocol {
    Injective,
    CosmWasm,
    Osmosis,
}

#[cfg(feature = "token_factory")]
impl Protocol {
    #![allow(dead_code)]
    #[allow(unreachable_code)]
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
    #[allow(unused_assignments)]
    fn as_str(&self) -> &'static str {
        match self {
            Self::Injective => "injective",
            Self::CosmWasm => "cosmwasm",
            Self::Osmosis => "osmosis",
        }
    }
}

#[allow(dead_code)]
pub(crate) enum MsgTypes {
    CreateDenom,
    Mint,
    Burn,
}

impl MsgTypes {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateDenom => "MsgCreateDenom",
            Self::Mint => "MsgMint",
            Self::Burn => "MsgBurn",
        }
    }
}

pub(crate) trait EncodeMessage {
    fn encode(sender: String, data: Self) -> Vec<u8>;
}
#[allow(dead_code)]
#[cfg(feature = "token_factory")]
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

use std::fmt::Debug;

use anyhow::Result as AnyResult;
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_schema::serde::de::DeserializeOwned;
use cosmwasm_std::{
    coin, coins, to_json_binary, Addr, Api, BankMsg, Binary, BlockInfo, CustomQuery, Querier,
    Storage, SubMsgResponse,
};
use cw_multi_test::{AppResponse, BankSudo, CosmosRouter, Stargate};

use white_whale_std::tokenfactory::burn::MsgBurn;
use white_whale_std::tokenfactory::common::EncodeMessage;
use white_whale_std::tokenfactory::create_denom::{MsgCreateDenom, MsgCreateDenomResponse};
use white_whale_std::tokenfactory::mint::MsgMint;
use white_whale_std::tokenfactory::responses::{Params, QueryParamsResponse};

pub struct StargateMock {}

impl Stargate for StargateMock {
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        type_url: String,
        value: Binary,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        match type_url.as_str() {
            "/osmosis.tokenfactory.v1beta1.MsgCreateDenom"
            | "/cosmwasm.tokenfactory.v1beta1.MsgCreateDenom"
            | "/injective.tokenfactory.v1beta1.MsgCreateDenom" => {
                let tf_msg: MsgCreateDenom = MsgCreateDenom::decode(value.into())?;
                let submsg_response = SubMsgResponse {
                    events: vec![],
                    data: Some(to_json_binary(&MsgCreateDenomResponse {
                        new_token_denom: format!("factory/{}/{}", tf_msg.sender, tf_msg.subdenom),
                    })?),
                };
                Ok(submsg_response.into())
            }
            "/osmosis.tokenfactory.v1beta1.MsgMint"
            | "/cosmwasm.tokenfactory.v1beta1.MsgMint"
            | "/injective.tokenfactory.v1beta1.MsgMint" => {
                let tf_msg: MsgMint = MsgMint::decode(value.into())?;
                let mint_coins = tf_msg.amount;
                let bank_sudo = BankSudo::Mint {
                    to_address: tf_msg.mint_to_address,
                    amount: coins(mint_coins.amount.u128(), mint_coins.denom),
                };
                router.sudo(api, storage, block, bank_sudo.into())
            }
            "/osmosis.tokenfactory.v1beta1.MsgBurn"
            | "/cosmwasm.tokenfactory.v1beta1.MsgBurn"
            | "/injective.tokenfactory.v1beta1.MsgBurn" => {
                let tf_msg: MsgBurn = MsgBurn::decode(value.into())?;
                let burn_coins = tf_msg.amount;
                let burn_msg = BankMsg::Burn {
                    amount: coins(burn_coins.amount.u128(), burn_coins.denom),
                };
                router.execute(
                    api,
                    storage,
                    block,
                    Addr::unchecked(tf_msg.sender),
                    burn_msg.into(),
                )
            }
            _ => Err(anyhow::anyhow!(
                "Unexpected exec msg {type_url} from {sender:?}",
            )),
        }
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        path: String,
        _data: Binary,
    ) -> AnyResult<Binary> {
        match path.as_str() {
            "/injective.tokenfactory.v1beta1.QueryParamsResponse" => {
                Ok(to_json_binary(&QueryParamsResponse {
                    params: Some(Params {
                        denom_creation_fee: vec![coin(1_000_000, "uosmo")],
                        denom_creation_gas_consume: 0,
                    }),
                })?)
            }
            _ => Err(anyhow::anyhow!("Unexpected stargate query request {path}",)),
        }
    }
}

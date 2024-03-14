use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

///QueryParamsResponse is the response type for the Query/Params RPC method.
#[cw_serde]
pub struct QueryParamsResponse {
    pub params: Option<Params>,
}

/// Params defines the parameters for the tokenfactory module.
#[cw_serde]
pub struct Params {
    pub denom_creation_fee: Vec<Coin>,
    pub denom_creation_gas_consume: u64,
}

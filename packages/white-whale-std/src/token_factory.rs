use cosmwasm_std::{CosmosMsg, StdResult};

/// A convenience function for processing the case where we use a token factory LP token or not.
pub fn is_token_factory_lp<F, G>(
    token_factory_lp: bool,
    on_true: F,
    on_false: G,
) -> StdResult<CosmosMsg>
where
    F: FnOnce() -> StdResult<CosmosMsg>,
    G: FnOnce() -> StdResult<CosmosMsg>,
{
    if token_factory_lp {
        on_true()
    } else {
        on_false()
    }
}

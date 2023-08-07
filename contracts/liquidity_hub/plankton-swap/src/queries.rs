use cosmwasm_std::{Deps, StdResult};
use white_whale::pool_network::factory::NativeTokenDecimalsResponse;

use crate::state::ALLOW_NATIVE_TOKENS;

/// Query the native token decimals
pub fn query_native_token_decimal(
    deps: Deps,
    denom: String,
) -> StdResult<NativeTokenDecimalsResponse> {
    let decimals = ALLOW_NATIVE_TOKENS.load(deps.storage, denom.as_bytes())?;

    Ok(NativeTokenDecimalsResponse { decimals })
}
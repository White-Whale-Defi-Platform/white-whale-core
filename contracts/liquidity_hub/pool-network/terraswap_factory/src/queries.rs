use crate::state::{pair_key, read_pairs, Config, ALLOW_NATIVE_TOKENS, CONFIG, PAIRS};
use cosmwasm_std::{Deps, StdResult};
use pool_network::asset::{AssetInfo, PairInfo, PairInfoRaw};
use pool_network::factory::{ConfigResponse, NativeTokenDecimalsResponse, PairsResponse};

/// Queries [Config]
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        token_code_id: config.token_code_id,
        pair_code_id: config.pair_code_id,
        fee_collector_addr: config.fee_collector_addr.to_string(),
    };

    Ok(resp)
}

/// Queries info about a given Pair
pub fn query_pair(deps: Deps, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let pair_info: PairInfoRaw = PAIRS.load(deps.storage, &pair_key)?;
    pair_info.to_normal(deps.api)
}

/// Queries all the pairs created by the factory
pub fn query_pairs(
    deps: Deps,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some([
            start_after[0].to_raw(deps.api)?,
            start_after[1].to_raw(deps.api)?,
        ])
    } else {
        None
    };

    let pairs: Vec<PairInfo> = read_pairs(deps.storage, deps.api, start_after, limit)?;
    let resp = PairsResponse { pairs };

    Ok(resp)
}

/// Query the native token decimals
pub fn query_native_token_decimal(
    deps: Deps,
    denom: String,
) -> StdResult<NativeTokenDecimalsResponse> {
    let decimals = ALLOW_NATIVE_TOKENS.load(deps.storage, denom.as_bytes())?;

    Ok(NativeTokenDecimalsResponse { decimals })
}

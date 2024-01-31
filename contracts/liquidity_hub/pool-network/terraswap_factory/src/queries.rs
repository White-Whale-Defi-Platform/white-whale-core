use crate::state::{
    pair_key, read_pairs, read_trios, trio_key, Config, ALLOW_NATIVE_TOKENS, CONFIG, PAIRS, TRIOS,
};
use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, StdResult};
use white_whale_std::pool_network::asset::{
    AssetInfo, PairInfo, PairInfoRaw, TrioInfo, TrioInfoRaw,
};
use white_whale_std::pool_network::factory::{
    ConfigResponse, NativeTokenDecimalsResponse, PairsResponse, TriosResponse,
};

/// Queries [Config]
pub fn query_config(deps: Deps<TerraQuery>) -> StdResult<ConfigResponse> {
    let config: Config = CONFIG.load(deps.storage)?;

    Ok(ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        token_code_id: config.token_code_id,
        pair_code_id: config.pair_code_id,
        trio_code_id: config.trio_code_id,
        fee_collector_addr: config.fee_collector_addr.to_string(),
    })
}

/// Queries info about a given Pair
pub fn query_pair(deps: Deps<TerraQuery>, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let pair_info: PairInfoRaw = PAIRS.load(deps.storage, &pair_key)?;
    pair_info.to_normal(deps.api)
}

/// Queries all the pairs created by the factory
pub fn query_pairs(
    deps: Deps<TerraQuery>,
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

/// Queries info about a given Trio
pub fn query_trio(deps: Deps<TerraQuery>, asset_infos: [AssetInfo; 3]) -> StdResult<TrioInfo> {
    let trio_key = trio_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
        asset_infos[2].to_raw(deps.api)?,
    ]);
    let trio_info: TrioInfoRaw = TRIOS.load(deps.storage, &trio_key)?;
    trio_info.to_normal(deps.api)
}

/// Queries all the trios created by the factory
pub fn query_trios(
    deps: Deps<TerraQuery>,
    start_after: Option<[AssetInfo; 3]>,
    limit: Option<u32>,
) -> StdResult<TriosResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some([
            start_after[0].to_raw(deps.api)?,
            start_after[1].to_raw(deps.api)?,
            start_after[2].to_raw(deps.api)?,
        ])
    } else {
        None
    };

    let trios: Vec<TrioInfo> = read_trios(deps.storage, deps.api, start_after, limit)?;
    let resp = TriosResponse { trios };

    Ok(resp)
}

/// Query the native token decimals
pub fn query_native_token_decimal(
    deps: Deps<TerraQuery>,
    denom: String,
) -> StdResult<NativeTokenDecimalsResponse> {
    let decimals = ALLOW_NATIVE_TOKENS.load(deps.storage, denom.as_bytes())?;

    Ok(NativeTokenDecimalsResponse { decimals })
}

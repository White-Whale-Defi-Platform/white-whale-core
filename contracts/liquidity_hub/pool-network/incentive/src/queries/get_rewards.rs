use cosmwasm_std::{Deps, Env, StdResult};
use white_whale::pool_network::{asset::Asset, incentive::GetRewardsResponse};

use crate::{
    claim::{calculate_claimable_amount, get_user_share},
    state::{FLOWS, LAST_CLAIMED_INDEX},
};

pub fn get_rewards(deps: Deps, env: Env, address: String) -> StdResult<GetRewardsResponse> {
    let address = deps.api.addr_validate(&address)?;

    let user_share = get_user_share(&deps, address.clone())?;

    let last_claim_time = LAST_CLAIMED_INDEX.may_load(deps.storage, address)?;

    let rewards = FLOWS
        .may_load(deps.storage)?
        .unwrap_or_default()
        .into_iter()
        .map(|flow| {
            let reward = calculate_claimable_amount(&flow, &env, last_claim_time, user_share)?;

            Ok(Asset {
                amount: reward,
                info: flow.flow_asset.info,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(GetRewardsResponse { rewards })
}

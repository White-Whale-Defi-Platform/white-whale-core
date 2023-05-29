use cosmwasm_std::{Deps, Env, StdResult};
use white_whale::pool_network::{asset::Asset, incentive::RewardsResponse};

use crate::{
    claim::{calculate_claimable_amount, get_user_share},
    state::{FLOWS, LAST_CLAIMED_INDEX},
};

pub fn get_rewards(deps: Deps, env: Env, address: String) -> StdResult<RewardsResponse> {
    let address = deps.api.addr_validate(&address)?;

    let user_share = get_user_share(&deps, address.clone())?;
    println!("user_share: {}", user_share);
    let last_claim_time = LAST_CLAIMED_INDEX
        .may_load(deps.storage, address)?
        //.unwrap_or(env.block.time.seconds());
        .unwrap_or(0u64);
    println!("last_claim_time: {}", last_claim_time);
    let rewards = FLOWS
        .may_load(deps.storage)?
        .unwrap_or_default()
        .into_iter()
        .filter(|flow| flow.start_timestamp <= env.block.time.seconds())
        .map(|flow| {
            println!("FLOOOOOOW: {:?}", flow);
            let reward = calculate_claimable_amount(&flow, &env, last_claim_time, user_share)?;

            Ok(Asset {
                amount: reward,
                info: flow.flow_asset.info,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    println!("rewards: {:?}", rewards);
    Ok(RewardsResponse { rewards })
}


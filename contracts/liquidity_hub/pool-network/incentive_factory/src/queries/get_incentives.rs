use classic_bindings::TerraQuery;
use cosmwasm_std::{Deps, Order, StdResult, Storage};
use cw_storage_plus::Bound;
use white_whale::pool_network::{
    asset::{AssetInfo, AssetInfoRaw},
    incentive_factory::{IncentivesContract, IncentivesResponse},
};

use crate::state::INCENTIVE_MAPPINGS;

/// Queries all the pairs created by the factory
pub fn get_incentives(
    deps: Deps<TerraQuery>,
    start_after: Option<AssetInfo>,
    limit: Option<u32>,
) -> StdResult<IncentivesResponse> {
    let start_after = start_after
        .map(|asset| asset.to_raw(deps.api))
        .transpose()?;

    read_incentives(deps.storage, start_after, limit)
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_incentives(
    storage: &dyn Storage,
    start_after: Option<AssetInfoRaw>,
    limit: Option<u32>,
) -> StdResult<IncentivesResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

    INCENTIVE_MAPPINGS
        .range(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (lp_reference, incentive_address) = item?;

            Ok(IncentivesContract {
                incentive_address,
                lp_reference,
            })
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<AssetInfoRaw>) -> Option<Vec<u8>> {
    start_after.map(|lp_info| {
        let mut v = lp_info.as_bytes().to_vec();
        v.push(1);
        v
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::mock_dependencies, to_binary, Addr, Binary, DepsMut, Reply, SubMsgResponse,
        SubMsgResult,
    };
    use protobuf::{Message, SpecialFields};
    use white_whale::pool_network::{asset::AssetInfo, incentive_factory::IncentivesContract};

    use crate::{
        reply::create_incentive_reply::{create_incentive_reply, CREATE_INCENTIVE_REPLY_ID},
        response::MsgInstantiateContractResponse,
    };

    use super::{get_incentives, MAX_LIMIT};

    /// Fires the `reply` message for `create_incentive`, in order to create a specific incentive of ID `id`.
    fn create_incentive(deps: DepsMut, id: u64) {
        create_incentive_reply(
            deps,
            Reply {
                id: CREATE_INCENTIVE_REPLY_ID,
                result: SubMsgResult::Ok(SubMsgResponse {
                    data: Some(Binary::from(
                        Message::write_to_bytes(&MsgInstantiateContractResponse {
                            address: format!("incentive{id}"),
                            data: to_binary(
                                &white_whale::pool_network::incentive::InstantiateReplyCallback {
                                    lp_asset: get_lp_asset(id),
                                },
                            )
                            .unwrap()
                            .as_slice()
                            .to_vec(),
                            special_fields: SpecialFields::new(),
                        })
                        .unwrap(),
                    )),
                    events: vec![],
                }),
            },
        )
        .unwrap();
    }

    /// Alternates between `NativeToken`'s and `Token`'s depending on if `id` is even or odd, respectively.
    fn get_lp_asset(id: u64) -> AssetInfo {
        match id % 2 == 0 {
            true => AssetInfo::NativeToken {
                denom: format!("lp{id}"),
            },
            false => AssetInfo::Token {
                contract_addr: format!("lp{id}"),
            },
        }
    }

    #[test]
    fn does_handle_no_incentives() {
        // should be able to handle a query when there is no incentives
        let deps = mock_dependencies();

        let incentives = get_incentives(deps.as_ref(), None, None).unwrap();
        assert!(incentives.is_empty());
    }

    #[test]
    fn does_return_incentives() {
        let mut deps = mock_dependencies();

        // create two incentive contracts
        create_incentive(deps.as_mut(), 1);
        create_incentive(deps.as_mut(), 2);

        let incentives = get_incentives(deps.as_ref(), None, None).unwrap();
        assert_eq!(
            incentives,
            vec![
                IncentivesContract {
                    incentive_address: Addr::unchecked("incentive1"),
                    lp_reference: get_lp_asset(1)
                        .to_raw(&deps.api)
                        .unwrap()
                        .as_bytes()
                        .to_vec()
                },
                IncentivesContract {
                    incentive_address: Addr::unchecked("incentive2"),
                    lp_reference: get_lp_asset(2)
                        .to_raw(&deps.api)
                        .unwrap()
                        .as_bytes()
                        .to_vec()
                }
            ]
        );
    }

    #[test]
    fn does_paginate() {
        let mut deps = mock_dependencies();

        // create two incentive contracts
        create_incentive(deps.as_mut(), 1);
        create_incentive(deps.as_mut(), 2);

        let incentives = get_incentives(deps.as_ref(), None, Some(1)).unwrap();
        assert_eq!(
            incentives,
            vec![IncentivesContract {
                incentive_address: Addr::unchecked("incentive1"),
                lp_reference: get_lp_asset(1)
                    .to_raw(&deps.api)
                    .unwrap()
                    .as_bytes()
                    .to_vec()
            },]
        );

        // asking again using the `start_after` option should now work.
        let incentives = get_incentives(deps.as_ref(), Some(get_lp_asset(1)), None).unwrap();
        assert_eq!(
            incentives,
            vec![IncentivesContract {
                incentive_address: Addr::unchecked("incentive2"),
                lp_reference: get_lp_asset(2)
                    .to_raw(&deps.api)
                    .unwrap()
                    .as_bytes()
                    .to_vec()
            }]
        );
    }

    #[test]
    fn does_start_after() {
        let mut deps = mock_dependencies();

        // create two incentive contracts
        create_incentive(deps.as_mut(), 1);
        create_incentive(deps.as_mut(), 2);

        let mut start_after = [get_lp_asset(1), get_lp_asset(2)]
            .map(|asset| asset.to_raw(&deps.api).unwrap().as_bytes().to_vec());
        start_after.sort_unstable();

        let incentives = get_incentives(deps.as_ref(), Some(get_lp_asset(1)), None).unwrap();
        assert_eq!(
            incentives,
            vec![IncentivesContract {
                incentive_address: Addr::unchecked("incentive2"),
                lp_reference: get_lp_asset(2)
                    .to_raw(&deps.api)
                    .unwrap()
                    .as_bytes()
                    .to_vec()
            },]
        );
    }

    #[test]
    fn does_have_max_limit() {
        let mut deps = mock_dependencies();

        // create MAX_LIMIT + 1 incentives
        (0..=MAX_LIMIT.into()).for_each(|id| create_incentive(deps.as_mut(), id));

        // when querying. we should only get MAX_LIMIT amount if we try to query more than possible
        let incentives = get_incentives(deps.as_ref(), None, Some(u32::MAX)).unwrap();
        assert_eq!(u32::try_from(incentives.len()).unwrap(), MAX_LIMIT);
    }
}

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::{Item, Map};

use terraswap::asset::Asset;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub staking_contract_addr: Addr,
    pub fee_collector_addr: Addr,
    pub grace_period: u128,
}

#[cw_serde]
pub struct Epoch {
    // Epoch identifier
    pub id: u128,
    // Initial fees to be distributed in this epoch.
    pub total: Vec<Asset>,
    // Fees left to be claimed on this epoch. These available fees are forwarded when the epoch expires.
    pub available: Vec<Asset>,
    // Fees that were claimed on this epoch. For keeping record on the total fees claimed.
    pub claimed: Vec<Asset>,
}

impl Epoch {
    // Returns the default, non-initialized version of an [Epoch].
    pub fn default() -> Self {
        Self {
            id: 0,
            total: vec![],
            available: vec![],
            claimed: vec![],
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const LAST_CLAIMED_EPOCH: Map<&Addr, u128> = Map::new("last_claimed_epoch");
pub const EPOCHS: Map<&[u8], Epoch> = Map::new("epochs");

/// Returns the current epoch, which is the last on the EPOCHS map.
pub fn get_current_epoch(deps: Deps) -> StdResult<Epoch> {
    let option = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .next();

    let epoch = match option {
        Some(Ok((_, epoch))) => epoch,
        _ => Epoch::default(),
    };

    Ok(epoch)
}

/// Returns the [Epoch] with the given id.
pub fn get_epoch(deps: Deps, id: u128) -> StdResult<Epoch> {
    let option = EPOCHS.may_load(deps.storage, &id.to_be_bytes())?;

    let epoch = match option {
        Some(epoch) => epoch,
        None => Epoch::default(),
    };

    Ok(epoch)
}

/// Returns the epoch that is falling out the grace period, which is the one expiring after creating
/// a new epoch is created.
pub fn get_expiring_epoch(deps: Deps) -> StdResult<Option<Epoch>> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    let option = EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take((grace_period + 1) as usize)
        .skip(grace_period as usize)
        .next();

    let epoch = option
        .and_then(|result| result.ok())
        .map(|(_, epoch)| epoch);

    Ok(epoch)
}

/// Returns the epochs that are within the grace period, i.e. the ones which fees can still be claimed.
/// The result is ordered by epoch id, descending. Thus, the first element is the current epoch.
pub fn get_claimable_epochs(deps: Deps) -> StdResult<Vec<Epoch>> {
    let grace_period = CONFIG.load(deps.storage)?.grace_period;

    EPOCHS
        .range(deps.storage, None, None, Order::Descending)
        .take((grace_period) as usize)
        .map(|item| {
            let (_, epoch) = item?;
            Ok(epoch)
        })
        .collect::<StdResult<Vec<Epoch>>>()
}

mod test {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Uint128;

    use terraswap::asset::{Asset, AssetInfo};

    use crate::contract::{execute, instantiate};
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use crate::state::{
        get_claimable_epochs, get_current_epoch, get_expiring_epoch, Epoch, EPOCHS,
    };

    // create test for get_current_epoch
    #[test]
    fn test_get_current_epoch() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            staking_contract_addr: "staking".to_string(),
            fee_collector_addr: "fee_collector".to_string(),
            grace_period: 2,
        };

        let info = mock_info("owner", &[]);

        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        EPOCHS
            .save(
                &mut deps.storage,
                &1_i32.to_be_bytes(),
                &Epoch {
                    id: 1,
                    total: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::from(10_000_000u128),
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uatom".to_string(),
                            },
                            amount: Uint128::from(10_000_000u128),
                        },
                    ],
                    available: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::from(1_000_000u128),
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uatom".to_string(),
                            },
                            amount: Uint128::from(7_000_000u128),
                        },
                    ],
                    claimed: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::from(9_000_000u128),
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uatom".to_string(),
                            },
                            amount: Uint128::from(3_000_000u128),
                        },
                    ],
                },
            )
            .unwrap();

        EPOCHS
            .save(
                &mut deps.storage,
                &2_i32.to_be_bytes(),
                &Epoch {
                    id: 2,
                    total: vec![Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::from(15_000_000u128),
                    }],
                    available: vec![Asset {
                        info: AssetInfo::NativeToken {
                            denom: "uwhale".to_string(),
                        },
                        amount: Uint128::from(15_000_000u128),
                    }],
                    claimed: vec![],
                },
            )
            .unwrap();

        EPOCHS
            .save(
                &mut deps.storage,
                &3_i32.to_be_bytes(),
                &Epoch {
                    id: 3,
                    total: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uatom".to_string(),
                            },
                            amount: Uint128::from(5_000_000u128),
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::from(5_000_000u128),
                        },
                    ],
                    available: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uatom".to_string(),
                            },
                            amount: Uint128::from(4_000_000u128),
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::from(4_000_000u128),
                        },
                    ],
                    claimed: vec![
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uatom".to_string(),
                            },
                            amount: Uint128::from(1_000_000u128),
                        },
                        Asset {
                            info: AssetInfo::NativeToken {
                                denom: "uwhale".to_string(),
                            },
                            amount: Uint128::from(1_000_000u128),
                        },
                    ],
                },
            )
            .unwrap();

        let current_epoch = get_current_epoch(deps.as_ref()).unwrap();
        let expiring_epoch = get_expiring_epoch(deps.as_ref()).unwrap();
        let claimable_epochs = get_claimable_epochs(deps.as_ref()).unwrap();

        println!("current epoch: {:?}", current_epoch);
        println!("expiring epoch: {:?}", expiring_epoch.unwrap());
        println!("claimable epoch: {:?}", claimable_epochs);

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("fee_collector", &[]),
            ExecuteMsg::Claim {},
        )
        .unwrap();

        println!("res: {:?}", res);
    }
}

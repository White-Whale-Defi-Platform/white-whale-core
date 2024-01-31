use cosmwasm_std::{coin, Uint128};
use std::collections::{HashMap, HashSet};
use white_whale_std::fee::Fee;
use white_whale_std::pool_network::asset::{Asset, AssetInfo};
use white_whale_std::traits::AssetReference;
use white_whale_std::vault_manager::{Vault, VaultFee};

#[test]
pub fn query_balances() {
    let mut balances = HashMap::new();

    let coins = vec![
        coin(1_000u128, "uwhale".to_string()),
        coin(1_000u128, "usdc".to_string()),
        coin(1_000u128, "uluna".to_string()),
        coin(1_000u128, "ibc/something".to_string()),
    ];

    for coin in coins {
        let asset_info = AssetInfo::NativeToken {
            denom: coin.denom.clone(),
        };
        balances.insert(asset_info.get_reference().to_vec(), coin.amount);
    }

    assert_eq!(balances.len(), 4);

    println!("balances::: {:?}", balances);

    let vaults = vec![
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(500),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "0".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "uwhale".to_string(),
                },
                amount: Uint128::new(500),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "2".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "usdc".to_string(),
                },
                amount: Uint128::new(1000),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "3".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(1000),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "4".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ibc/something".to_string(),
                },
                amount: Uint128::new(250),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "5".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ibc/something".to_string(),
                },
                amount: Uint128::new(250),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "6".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ibc/something".to_string(),
                },
                amount: Uint128::new(250),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "7".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::NativeToken {
                    denom: "ibc/something".to_string(),
                },
                amount: Uint128::new(250),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "8".to_string(),
        },
        Vault {
            asset: Asset {
                info: AssetInfo::Token {
                    contract_addr: "cw20_contract".to_string(),
                },
                amount: Uint128::new(3333),
            },
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something".to_string(),
            },
            fees: VaultFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                flash_loan_fee: Fee {
                    share: Default::default(),
                },
            },
            identifier: "9".to_string(),
        },
    ];

    let mut encountered_assets: HashSet<Vec<u8>> = HashSet::new();

    let filtered_vaults: Vec<Vault> = vaults
        .into_iter()
        .filter(|vault| {
            let is_duplicate =
                !encountered_assets.insert(vault.asset.info.clone().get_reference().to_vec());
            !is_duplicate && !balances.contains_key(vault.asset.info.get_reference())
        })
        .collect();

    println!("filtered_vaults:::: {:?}", filtered_vaults);
    assert_eq!(filtered_vaults.len(), 1);

    for vault in filtered_vaults {
        let balance = vault.asset.amount;
        balances.insert(vault.asset.info.get_reference().to_vec(), balance);
    }

    println!("balances:::: {:?}", balances);

    assert_eq!(balances.len(), 5);
}

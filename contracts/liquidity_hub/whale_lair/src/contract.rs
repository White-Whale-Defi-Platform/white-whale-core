use cosmwasm_std::{Addr, entry_point};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale::pool_network::asset::AssetInfo;
use white_whale::whale_lair::{Config, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::{commands, migrations, queries};
use crate::error::ContractError;
use crate::helpers::validate_growth_rate;
use crate::state::{BONDING_ASSETS_LIMIT, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-whale_lair";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if msg.bonding_assets.len() > BONDING_ASSETS_LIMIT {
        return Err(ContractError::InvalidBondingAssetsLimit(
            BONDING_ASSETS_LIMIT,
            msg.bonding_assets.len(),
        ));
    }

    validate_growth_rate(msg.growth_rate)?;

    //todo since this should only accept native tokens, we could omit the asset type and pass the denom directly
    for asset in &msg.bonding_assets {
        match asset {
            AssetInfo::Token { .. } => return Err(ContractError::InvalidBondingAsset {}),
            AssetInfo::NativeToken { .. } => {}
        };
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        unbonding_period: msg.unbonding_period,
        growth_rate: msg.growth_rate,
        bonding_assets: msg.bonding_assets,
        fee_distributor_addr: Addr::unchecked(""),
    };

    CONFIG.save(deps.storage, &config)?;

    let bonding_assets = config
        .bonding_assets
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Ok(Response::default().add_attributes(vec![
        ("action", "instantiate".to_string()),
        ("owner", config.owner.to_string()),
        ("unbonding_period", config.unbonding_period.to_string()),
        ("growth_rate", config.growth_rate.to_string()),
        ("bonding_assets", bonding_assets),
    ]))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Bond { asset } => commands::bond(deps, env.block.time, info, env, asset),
        ExecuteMsg::Unbond { asset } => commands::unbond(deps, env.block.time, info, env, asset),
        ExecuteMsg::Withdraw { denom } => {
            commands::withdraw(deps, env.block.time, info.sender, denom)
        }
        ExecuteMsg::UpdateConfig {
            owner,
            unbonding_period,
            growth_rate,
            fee_distributor_addr,
        } => commands::update_config(
            deps,
            info,
            owner,
            unbonding_period,
            growth_rate,
            fee_distributor_addr,
        ),
        ExecuteMsg::FillRewards { assets } => Ok(Response::default().add_attributes(vec![
            ("action", "fill_rewards".to_string())
        ])),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Bonded { address } => to_binary(&queries::query_bonded(deps, address)?),
        QueryMsg::Unbonding {
            address,
            denom,
            start_after,
            limit,
        } => to_binary(&queries::query_unbonding(
            deps,
            address,
            denom,
            start_after,
            limit,
        )?),
        QueryMsg::Withdrawable { address, denom } => to_binary(&queries::query_withdrawable(
            deps,
            env.block.time,
            address,
            denom,
        )?),
        QueryMsg::Weight {
            address,
            timestamp,
            global_index,
        } => {
            // If timestamp is not provided, use current block time
            let timestamp = timestamp.unwrap_or(env.block.time);

            // TODO: Make better timestamp handling
            to_binary(&queries::query_weight(
                deps,
                timestamp,
                address,
                global_index,
            )?)
        }
        QueryMsg::TotalBonded {} => to_binary(&queries::query_total_bonded(deps)?),
        QueryMsg::GlobalIndex {} => to_binary(&queries::query_global_index(deps)?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use white_whale::migrate_guards::check_contract_name;

    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(ContractError::MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    if storage_version < Version::parse("0.9.0")? {
        migrations::migrate_to_v090(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

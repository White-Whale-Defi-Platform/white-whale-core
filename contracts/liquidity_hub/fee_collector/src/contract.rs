#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BalanceResponse, BankMsg, BankQuery, Binary, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QueryRequest, Reply, Response, StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use white_whale::fee_collector::{
    Config, ExecuteMsg, ForwardFeesResponse, InstantiateMsg, MigrateMsg, QueryMsg,
};
use white_whale::pool_network::asset::{Asset, AssetInfo, ToCoins};

use crate::error::ContractError;
use crate::queries::query_distribution_asset;
use crate::state::{CONFIG, TMP_EPOCH};
use crate::ContractError::MigrateInvalidVersion;
use crate::{commands, migrations, queries};

const CONTRACT_NAME: &str = "white_whale-fee_collector";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const FEES_COLLECTION_REPLY_ID: u64 = 1u64;
pub(crate) const FEES_AGGREGATION_REPLY_ID: u64 = 2u64;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(info.sender.as_str())?,
        pool_router: Addr::unchecked(""),
        fee_distributor: Addr::unchecked(""),
        pool_factory: Addr::unchecked(""),
        vault_factory: Addr::unchecked(""),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", config.owner.as_str()))
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.id == FEES_AGGREGATION_REPLY_ID {
        let mut epoch = TMP_EPOCH
            .may_load(deps.storage)?
            .ok_or(ContractError::CannotReadEpoch {})?;

        let asset_info = query_distribution_asset(deps.as_ref())?;

        let token_balance: Uint128 = match asset_info.clone() {
            AssetInfo::Token { .. } => {
                return Err(ContractError::InvalidContractsFeeAggregation {})
            }
            AssetInfo::NativeToken { denom } => {
                let balance_response: BalanceResponse =
                    deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
                        address: env.contract.address.to_string(),
                        denom,
                    }))?;
                balance_response.amount.amount
            }
        };

        let mut messages = vec![];

        // if not zero, it means there were fees aggregated
        if !token_balance.is_zero() {
            let fees = vec![Asset {
                info: asset_info,
                amount: token_balance,
            }];

            epoch.total = fees.clone();
            epoch.available = fees.clone();

            // send tokens to fee distributor
            let config = CONFIG.load(deps.storage)?;
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: config.fee_distributor.to_string(),
                amount: fees.to_coins()?,
            }));
        }

        TMP_EPOCH.remove(deps.storage);

        Ok(Response::default()
            .add_attribute("action", "reply")
            .add_attribute("new_epoch", epoch.to_string())
            .add_messages(messages)
            .set_data(to_binary(&ForwardFeesResponse { epoch })?))
    } else {
        Err(ContractError::UnknownReplyId(msg.id))
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CollectFees { collect_fees_for } => {
            commands::collect_fees(deps, info, env, collect_fees_for)
        }
        ExecuteMsg::UpdateConfig {
            owner,
            pool_router,
            fee_distributor,
            pool_factory,
            vault_factory,
        } => commands::update_config(
            deps,
            info,
            owner,
            pool_router,
            fee_distributor,
            pool_factory,
            vault_factory,
        ),
        ExecuteMsg::AggregateFees { aggregate_fees_for } => {
            commands::aggregate_fees(deps, env, aggregate_fees_for)
        }
        ExecuteMsg::ForwardFees { epoch, .. } => commands::forward_fees(deps, info, env, epoch),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::Fees {
            query_fees_for,
            all_time,
        } => to_binary(&queries::query_fees(
            deps,
            query_fees_for,
            all_time.unwrap_or(false),
        )?),
    }
}

#[cfg(not(tarpaulin_include))]
#[entry_point]
pub fn migrate(mut deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    use white_whale::migrate_guards::check_contract_name;

    let version: Version = CONTRACT_VERSION.parse()?;
    let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

    if storage_version >= version {
        return Err(MigrateInvalidVersion {
            current_version: storage_version,
            new_version: version,
        });
    }

    check_contract_name(deps.storage, CONTRACT_NAME.to_string())?;

    if storage_version <= Version::parse("1.0.5")? {
        migrations::migrate_to_v110(deps.branch())?;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

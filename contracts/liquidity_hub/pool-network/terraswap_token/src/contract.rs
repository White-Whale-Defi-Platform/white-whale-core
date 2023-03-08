#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw2::set_contract_version;
use cw20_base::contract::{create_accounts, execute as cw20_execute, query as cw20_query};
use cw20_base::msg::{ExecuteMsg, QueryMsg};
use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO};
use cw20_base::ContractError;

use pool_network::token::InstantiateMsg;

// version info for migration info
const CONTRACT_NAME: &str = "white_whale-cw20_token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // check valid token info
    msg.validate()?;

    // create initial accounts
    let total_supply = create_accounts(&mut deps, &msg.initial_balances)?;

    if let Some(limit) = msg.get_cap() {
        if total_supply > limit {
            return Err(ContractError::Std(StdError::generic_err(
                "Initial supply greater than cap",
            )));
        }
    }

    let mint = match msg.mint {
        Some(m) => Some(MinterData {
            minter: deps.api.addr_validate(&m.minter)?,
            cap: m.cap,
        }),
        None => None,
    };

    // store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
        mint,
    };

    TOKEN_INFO.save(deps.storage, &data)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    cw20_execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    cw20_query(deps, env, msg)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Uint128;
    use cw20::{Cw20Coin, MinterResponse};

    use super::*;

    #[test]
    fn invalid_supply_limit() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            name: "test".to_string(),
            symbol: "test".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: "owner".to_string(),
                amount: Uint128::new(100),
            }],
            mint: Some(MinterResponse {
                minter: "minter_addr".to_string(),
                cap: Some(Uint128::new(99)),
            }),
        };
        let info = mock_info("creator", &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg);
        match res {
            Ok(_) => panic!("should error"),
            Err(ContractError::Std(error)) => assert_eq!(
                error,
                StdError::generic_err("Initial supply greater than cap",)
            ),
            _ => panic!("should error"),
        }
    }
    #[test]
    fn no_minter_data() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            name: "test".to_string(),
            symbol: "test".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: "owner".to_string(),
                amount: Uint128::new(100),
            }],
            mint: None,
        };
        let info = mock_info("creator", &[]);

        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    }
}

use cosmwasm_std::{from_binary, DepsMut, Env, MessageInfo, Response};

use white_whale::pool_network::asset::AssetInfo;
use white_whale::vault_network::vault::{Cw20HookMsg, Cw20ReceiveMsg};
use withdraw::withdraw;

use crate::{error::VaultError, state::CONFIG};

pub mod withdraw;

/// Handles receiving CW20 messages
pub fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, VaultError> {
    // callback can only be called by liquidity token
    let config = CONFIG.load(deps.storage)?;

    let cw20_lp_token = match config.lp_asset {
        AssetInfo::Token { contract_addr } => contract_addr,
        AssetInfo::NativeToken { .. } => return Err(VaultError::Unauthorized {}),
    };

    if info.sender != deps.api.addr_validate(cw20_lp_token.as_str())? {
        return Err(VaultError::ExternalCallback {});
    }

    match from_binary(&msg.msg)? {
        Cw20HookMsg::Withdraw {} => withdraw(deps, env, msg.sender, msg.amount),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{to_binary, Addr, Uint128};

    #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
    use cosmwasm_std::testing::mock_info;

    use white_whale::pool_network::asset::AssetInfo;
    use white_whale::vault_network::vault::Config;

    use crate::state::CONFIG;
    use crate::tests::get_fees;
    use crate::{
        error::VaultError,
        tests::{mock_creator, mock_instantiate::mock_instantiate},
    };

    use super::receive;

    #[test]
    fn cannot_receive_from_native_token() {
        let (mut deps, env) = mock_instantiate(
            1,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            false,
        );

        let config = Config {
            owner: mock_creator().sender,
            lp_asset: AssetInfo::NativeToken {
                denom: "factory/something/uLP".to_string(),
            },
            asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            deposit_enabled: true,
            flash_loan_enabled: true,
            withdraw_enabled: true,
            fee_collector_addr: Addr::unchecked("fee_collector"),
            fees: get_fees(),
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let res = receive(
            deps.as_mut(),
            env,
            mock_creator(),
            white_whale::vault_network::vault::Cw20ReceiveMsg {
                sender: mock_creator().sender.into_string(),
                amount: Uint128::new(5_000),
                msg: to_binary(&white_whale::vault_network::vault::Cw20HookMsg::Withdraw {})
                    .unwrap(),
            },
        );

        assert_eq!(res.unwrap_err(), VaultError::Unauthorized {})
    }

    #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
    #[test]
    fn cannot_receive_from_not_liquidity_token() {
        let (mut deps, env) = mock_instantiate(
            1,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            true,
        );

        let config = Config {
            owner: mock_creator().sender,
            lp_asset: AssetInfo::Token {
                contract_addr: "lp_token".to_string(),
            },
            asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            deposit_enabled: true,
            flash_loan_enabled: true,
            withdraw_enabled: true,
            fee_collector_addr: Addr::unchecked("fee_collector"),
            fees: get_fees(),
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let res = receive(
            deps.as_mut(),
            env,
            mock_info("lp_token_2", &[]), //wrong cw20 LP token
            white_whale::vault_network::vault::Cw20ReceiveMsg {
                sender: mock_creator().sender.into_string(),
                amount: Uint128::new(5_000),
                msg: to_binary(&white_whale::vault_network::vault::Cw20HookMsg::Withdraw {})
                    .unwrap(),
            },
        );

        assert_eq!(res.unwrap_err(), VaultError::ExternalCallback {})
    }
}

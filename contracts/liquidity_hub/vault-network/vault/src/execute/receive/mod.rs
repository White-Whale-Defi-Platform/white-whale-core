use cosmwasm_std::{from_binary, DepsMut, Env, MessageInfo, Response};
use vault_network::vault::{Cw20HookMsg, Cw20ReceiveMsg};

use crate::{error::VaultError, state::CONFIG};

mod withdraw;

use withdraw::withdraw;

/// Handles receiving CW20 messages
pub fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, VaultError> {
    // callback can only be called by liquidity token
    let config = CONFIG.load(deps.storage)?;

    if info.sender != config.liquidity_token {
        return Err(VaultError::ExternalCallback {});
    }

    match from_binary(&msg.msg)? {
        Cw20HookMsg::Withdraw {} => withdraw(deps, env, msg.sender, msg.amount),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{to_binary, Uint128};

    use crate::{
        error::VaultError,
        tests::{mock_creator, mock_instantiate::mock_instantiate},
    };

    use super::receive;

    #[test]
    fn cannot_receive_from_not_liquidity_token() {
        let (mut deps, env) = mock_instantiate(
            1,
            pool_network::asset::AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        );

        let res = receive(
            deps.as_mut(),
            env,
            mock_creator(),
            vault_network::vault::Cw20ReceiveMsg {
                sender: mock_creator().sender.into_string(),
                amount: Uint128::new(5_000),
                msg: to_binary(&vault_network::vault::Cw20HookMsg::Withdraw {}).unwrap(),
            },
        );

        assert_eq!(res.unwrap_err(), VaultError::ExternalCallback {})
    }
}

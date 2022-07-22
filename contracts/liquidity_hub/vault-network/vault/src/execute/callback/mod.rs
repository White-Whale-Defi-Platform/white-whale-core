mod after_trade;

pub use after_trade::after_trade;

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use vault_network::vault::CallbackMsg;

pub fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> StdResult<Response> {
    // callback can only be called by contract
    if info.sender != env.contract.address {
        return Err(StdError::GenericErr {
            msg: "Attempt to call callback function outside contract".to_string(),
        });
    }

    match msg {
        CallbackMsg::AfterTrade { old_balance } => after_trade(deps, env, old_balance),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{StdError, Uint128};
    use cw_multi_test::Executor;
    use terraswap::asset::AssetInfo;

    use crate::tests::{
        mock_app::mock_app,
        mock_creator, mock_execute,
        store_code::{store_cw20_token_code, store_vault_code},
    };

    #[test]
    fn does_fail_on_outside_request() {
        let (res, ..) = mock_execute(
            5,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            vault_network::vault::ExecuteMsg::Callback(
                vault_network::vault::CallbackMsg::AfterTrade {
                    old_balance: Uint128::new(5_000),
                },
            ),
        );

        assert_eq!(
            res.unwrap_err(),
            StdError::generic_err("Attempt to call callback function outside contract")
        )
    }

    #[test]
    fn does_succeed_on_internal_request() {
        let mut app = mock_app();

        let vault_id = store_vault_code(&mut app);
        let token_id = store_cw20_token_code(&mut app);

        // instantiate contract
        let vault_addr = app
            .instantiate_contract(
                vault_id,
                mock_creator().sender,
                &vault_network::vault::InstantiateMsg {
                    owner: mock_creator().sender.into_string(),
                    token_id,
                    asset_info: AssetInfo::NativeToken {
                        denom: "uluna".to_string(),
                    },
                },
                &[],
                "vault",
                None,
            )
            .unwrap();

        // execute contract with vault as sender of message
        app.execute_contract(
            vault_addr.clone(),
            vault_addr,
            &vault_network::vault::ExecuteMsg::Callback(
                vault_network::vault::CallbackMsg::AfterTrade {
                    old_balance: Uint128::new(0),
                },
            ),
            &[],
        )
        .unwrap();
    }
}

mod after_trade;

pub use after_trade::after_trade;

use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use vault_network::vault::CallbackMsg;

use crate::error::VaultError;

pub fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CallbackMsg,
) -> Result<Response, VaultError> {
    // callback can only be called by contract
    if info.sender != env.contract.address {
        return Err(VaultError::ExternalCallback {});
    }

    match msg {
        CallbackMsg::AfterTrade {
            old_balance,
            loan_amount,
        } => after_trade(deps, env, old_balance, loan_amount),
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{coins, Uint128};
    use cw_multi_test::Executor;
    use pool_network::asset::AssetInfo;

    use crate::{
        error::VaultError,
        tests::{
            get_fees,
            mock_app::mock_app_with_balance,
            mock_creator, mock_execute,
            store_code::{store_cw20_token_code, store_vault_code},
        },
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
                    loan_amount: Uint128::new(2_500),
                },
            ),
        );

        assert_eq!(res.unwrap_err(), VaultError::ExternalCallback {})
    }

    #[test]
    fn does_succeed_on_internal_request() {
        let mut app = mock_app_with_balance(vec![(mock_creator().sender, coins(1_000, "uluna"))]);

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
                    fee_collector_addr: "fee_collector".to_string(),
                    vault_fees: get_fees(),
                },
                &coins(1_000, "uluna"),
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
                    loan_amount: Uint128::new(1_000),
                },
            ),
            &[],
        )
        .unwrap();
    }
}

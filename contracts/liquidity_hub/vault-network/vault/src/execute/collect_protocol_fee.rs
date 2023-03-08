use cosmwasm_std::{attr, CosmosMsg, DepsMut, Response, Uint128};
use pool_network::asset::Asset;

use crate::{
    error::VaultError,
    state::{COLLECTED_PROTOCOL_FEES, CONFIG},
};

/// Collects all protocol fees accrued by the vault
pub fn collect_protocol_fees(deps: DepsMut) -> Result<Response, VaultError> {
    let config = CONFIG.load(deps.storage)?;

    // get the collected protocol fees so far
    let protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;

    // reset the collected protocol fees
    COLLECTED_PROTOCOL_FEES.save(
        deps.storage,
        &Asset {
            amount: Uint128::zero(),
            info: protocol_fees.info.clone(),
        },
    )?;

    let mut messages: Vec<CosmosMsg> = Vec::new();
    if protocol_fees.amount != Uint128::zero() {
        messages.push(protocol_fees.clone().into_msg(config.fee_collector_addr)?);
    }

    Ok(Response::new()
        .add_attributes(vec![
            attr("method", "collect_protocol_fees"),
            attr("amount", protocol_fees.amount.to_string()),
        ])
        .add_messages(messages))
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{attr, coins, BankMsg, Response, StdError, Uint128};
    use pool_network::asset::{Asset, AssetInfo};
    use vault_network::vault::ExecuteMsg;

    use crate::{
        contract::execute,
        state::COLLECTED_PROTOCOL_FEES,
        tests::{mock_creator, mock_instantiate::mock_instantiate},
    };

    #[test]
    fn can_collect_fees() {
        let asset = AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        let (mut deps, env) = mock_instantiate(1, asset);

        // inject protocol fees
        COLLECTED_PROTOCOL_FEES
            .update::<_, StdError>(&mut deps.storage, |mut fees| {
                fees.amount = Uint128::new(1_000);
                Ok(fees)
            })
            .unwrap();

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            ExecuteMsg::CollectProtocolFees {},
        )
        .unwrap();
        assert_eq!(
            res,
            Response::new()
                .add_attributes(vec![
                    attr("method", "collect_protocol_fees"),
                    attr("amount", "1000")
                ])
                .add_message(BankMsg::Send {
                    to_address: "fee_collector".to_string(),
                    amount: coins(1000, "uluna")
                })
        );
    }

    #[test]
    fn does_not_send_if_empty() {
        let (mut deps, env) = mock_instantiate(
            1,
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        );

        let res = execute(
            deps.as_mut(),
            env,
            mock_creator(),
            ExecuteMsg::CollectProtocolFees {},
        )
        .unwrap();
        assert_eq!(
            res,
            Response::new().add_attributes(vec![
                attr("method", "collect_protocol_fees"),
                attr("amount", "0")
            ])
        );
    }

    #[test]
    fn does_reset_state() {
        let asset = AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        };

        let (mut deps, env) = mock_instantiate(1, asset.clone());

        // inject protocol fees
        COLLECTED_PROTOCOL_FEES
            .update::<_, StdError>(&mut deps.storage, |mut fees| {
                fees.amount = Uint128::new(1_000);
                Ok(fees)
            })
            .unwrap();

        execute(
            deps.as_mut(),
            env,
            mock_creator(),
            ExecuteMsg::CollectProtocolFees {},
        )
        .unwrap();

        let collected_fees = COLLECTED_PROTOCOL_FEES.load(&deps.storage).unwrap();
        assert_eq!(
            collected_fees,
            Asset {
                amount: Uint128::zero(),
                info: asset
            }
        )
    }
}

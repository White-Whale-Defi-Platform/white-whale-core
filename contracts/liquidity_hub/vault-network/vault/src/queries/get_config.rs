use cosmwasm_std::{to_binary, Binary, Deps};

use crate::error::VaultError;
use crate::state::CONFIG;

pub fn get_config(deps: Deps) -> Result<Binary, VaultError> {
    Ok(to_binary(&CONFIG.load(deps.storage)?)?)
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env},
        Addr,
    };
    use pool_network::asset::AssetInfo;
    use vault_network::vault::Config;

    use crate::{
        contract::query,
        state::CONFIG,
        tests::{get_fees, mock_creator},
    };

    #[test]
    fn does_get_config() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let config = Config {
            owner: mock_creator().sender,
            liquidity_token: Addr::unchecked("lp_token"),
            asset_info: AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            deposit_enabled: false,
            flash_loan_enabled: true,
            withdraw_enabled: false,
            fee_collector_addr: Addr::unchecked("fee_collector"),
            fees: get_fees(),
        };

        CONFIG.save(&mut deps.storage, &config).unwrap();

        let res: Config = from_binary(
            &query(
                deps.as_ref(),
                env,
                vault_network::vault::QueryMsg::Config {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(res, config);
    }
}

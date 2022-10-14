use cosmwasm_std::{to_binary, Binary, Deps};

use crate::{err::StdResult, state::CONFIG};

/// Retrieves the contract configuration stored in state.
pub fn get_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;

    Ok(to_binary(&config)?)
}

#[cfg(test)]
mod test {
    use cosmwasm_std::Addr;
    use vault_network::vault_factory::{Config, QueryMsg};

    use crate::tests::{mock_creator, mock_query};

    #[test]
    fn does_get_config() {
        let (config, ..) = mock_query::<Config>(5, 6, QueryMsg::Config {});
        assert_eq!(
            config,
            Config {
                owner: mock_creator().sender,
                vault_id: 5,
                token_id: 6,
                fee_collector_addr: Addr::unchecked("fee_collector")
            }
        )
    }
}

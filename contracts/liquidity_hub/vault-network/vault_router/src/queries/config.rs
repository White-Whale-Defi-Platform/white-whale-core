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
    use vault_network::vault_router::{Config, QueryMsg};

    use crate::tests::{mock_creator, mock_query};

    #[test]
    fn does_get_config() {
        let (config, ..) = mock_query::<Config>("factory_addr".to_string(), QueryMsg::Config {});

        assert_eq!(
            config,
            Config {
                owner: mock_creator().sender,
                vault_factory: Addr::unchecked("factory_addr"),
            }
        );
    }
}

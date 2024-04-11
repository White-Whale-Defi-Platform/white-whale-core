use cosmwasm_std::{
    attr, coins, ensure, Attribute, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128,
};

use white_whale_std::lp_common::LP_SYMBOL;
use white_whale_std::tokenfactory;
use white_whale_std::vault_manager::{Vault, VaultFee};
use white_whale_std::whale_lair::fill_rewards_msg_coin;

use crate::state::{get_vault_by_identifier, CONFIG, VAULTS, VAULT_COUNTER};
use crate::ContractError;

/// Creates a new vault
pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_denom: String,
    fees: VaultFee,
    vault_identifier: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // verify fee payment
    let fee = cw_utils::must_pay(&info, config.vault_creation_fee.denom.as_str())?;
    ensure!(
        fee >= config.vault_creation_fee.amount,
        ContractError::InvalidVaultCreationFee {
            amount: fee,
            expected: config.vault_creation_fee.amount,
        }
    );

    let mut messages: Vec<CosmosMsg> = vec![];

    // send vault creation fee to whale lair
    let creation_fee = coins(
        config.vault_creation_fee.amount.u128(),
        config.vault_creation_fee.denom.clone(),
    );

    // send protocol fee to whale lair
    messages.push(fill_rewards_msg_coin(
        config.whale_lair_addr.into_string(),
        creation_fee,
    )?);

    let vault_id = VAULT_COUNTER.load(deps.storage)?;
    // if no identifier is provided, use the vault counter (id) as identifier
    let identifier = vault_identifier.unwrap_or(vault_id.to_string());

    // check if there is an existing vault with the given identifier
    let vault = get_vault_by_identifier(&deps.as_ref(), identifier.clone());
    ensure!(
        vault.is_err(),
        ContractError::ExistingVault {
            asset_denom,
            identifier
        }
    );

    // check the vault fees are valid
    fees.is_valid()?;

    let asset_label = white_whale_std::coin::get_label(&asset_denom)?;

    let mut attributes = Vec::<Attribute>::new();
    attributes.push(attr("vault_identifier", identifier.clone()));

    let lp_symbol = format!("{asset_label}.vault.{identifier}.{LP_SYMBOL}");
    let lp_denom = format!("{}/{}/{}", "factory", env.contract.address, lp_symbol);

    VAULTS.save(
        deps.storage,
        identifier.clone(),
        &Vault {
            asset: Coin {
                denom: asset_denom,
                amount: Uint128::zero(),
            },
            lp_denom: lp_denom.clone(),
            fees,
            identifier,
        },
    )?;

    attributes.push(attr("lp_denom", lp_denom));

    let message = tokenfactory::create_denom::create_denom(env.contract.address, lp_symbol);

    // increase vault counter
    VAULT_COUNTER.update(deps.storage, |mut counter| -> Result<_, ContractError> {
        counter += 1;
        Ok(counter)
    })?;

    messages.push(message);

    Ok(Response::default()
        .add_messages(messages)
        .add_attribute("action", "create_vault".to_string())
        .add_attributes(attributes))
}

/// Updates the manager config
#[allow(clippy::too_many_arguments)]
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    whale_lair_addr: Option<String>,
    vault_creation_fee: Option<Coin>,
    flash_loan_enabled: Option<bool>,
    deposit_enabled: Option<bool>,
    withdraw_enabled: Option<bool>,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let new_config = CONFIG.update::<_, ContractError>(deps.storage, |mut config| {
        if let Some(new_whale_lair_addr) = whale_lair_addr {
            config.whale_lair_addr = deps.api.addr_validate(&new_whale_lair_addr)?;
        }

        if let Some(vault_creation_fee) = vault_creation_fee {
            config.vault_creation_fee = vault_creation_fee;
        }

        if let Some(flash_loan_enabled) = flash_loan_enabled {
            config.flash_loan_enabled = flash_loan_enabled;
        }

        if let Some(deposit_enabled) = deposit_enabled {
            config.deposit_enabled = deposit_enabled;
        }

        if let Some(withdraw_enabled) = withdraw_enabled {
            config.withdraw_enabled = withdraw_enabled;
        }

        Ok(config)
    })?;

    Ok(Response::default().add_attributes(vec![
        ("method", "update_manager_config"),
        ("whale_lair_addr", &new_config.whale_lair_addr.into_string()),
        (
            "vault_creation_fee",
            &new_config.vault_creation_fee.to_string(),
        ),
        (
            "flash_loan_enabled",
            &new_config.flash_loan_enabled.to_string(),
        ),
        ("deposit_enabled", &new_config.deposit_enabled.to_string()),
        ("withdraw_enabled", &new_config.withdraw_enabled.to_string()),
    ]))
}

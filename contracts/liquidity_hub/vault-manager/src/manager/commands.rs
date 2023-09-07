use cosmwasm_std::{
    attr, instantiate2_address, to_binary, Api, Attribute, Binary, CodeInfoResponse, CosmosMsg,
    DepsMut, Env, MessageInfo, Response, StdError, WasmMsg,
};
use cw20::MinterResponse;

use white_whale::common::validate_owner;
use white_whale::constants::LP_SYMBOL;
use white_whale::pool_network::asset::{Asset, AssetInfo};
#[cfg(feature = "token_factory")]
use white_whale::pool_network::denom::MsgCreateDenom;
#[cfg(feature = "osmosis_token_factory")]
use white_whale::pool_network::denom_osmosis::MsgCreateDenom;
use white_whale::pool_network::token::InstantiateMsg as TokenInstantiateMsg;
use white_whale::traits::AssetReference;
use white_whale::vault_manager::{LpTokenType, Vault, VaultFee};

use crate::state::{COLLECTED_PROTOCOL_FEES, MANAGER_CONFIG, OWNER, VAULTS};
use crate::ContractError;

/// Creates a new vault
pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo,
    fees: VaultFee,
) -> Result<Response, ContractError> {
    let manager_config = MANAGER_CONFIG.load(deps.storage)?;

    let denom = match manager_config.vault_creation_fee.info {
        // this will never happen as the fee is always native, enforced when instantiating the contract
        AssetInfo::Token { .. } => "".to_string(),
        AssetInfo::NativeToken { denom } => denom,
    };

    // verify fee payment
    let amount = cw_utils::must_pay(&info, denom.as_str())?;
    if amount < manager_config.vault_creation_fee.amount {
        return Err(ContractError::InvalidVaultCreationFee {
            amount,
            expected: manager_config.vault_creation_fee.amount,
        });
    }

    //todo send vault creation fee to "fee collector", i.e. with hook directly to the whale lair
    // or whatever distributes the protocol fees

    let binding = asset_info.clone();
    let asset_info_reference = binding.get_reference();

    // check that existing vault does not exist
    let vault = VAULTS.may_load(deps.storage, asset_info_reference)?;
    if let Some(_) = vault {
        return Err(ContractError::ExistingVault { asset_info });
    }

    // check the fees are valid
    fees.is_valid()?;

    let asset_label = asset_info.clone().get_label(&deps.as_ref())?;
    let mut attributes = Vec::<Attribute>::new();

    let message = if manager_config.lp_token_type == LpTokenType::TokenFactory {
        #[cfg(all(not(feature = "token_factory"), not(feature = "osmosis_token_factory")))]
        return Err(ContractError::TokenFactoryNotEnabled {});

        let lp_symbol = format!("{asset_label}.vault.{LP_SYMBOL}");
        let denom = format!("{}/{}/{}", "factory", env.contract.address, lp_symbol);
        let lp_asset = AssetInfo::NativeToken { denom };

        VAULTS.save(
            deps.storage,
            asset_info_reference,
            &Vault {
                asset_info,
                lp_asset: lp_asset.clone(),
                fees,
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
        Ok(<MsgCreateDenom as Into<CosmosMsg>>::into(MsgCreateDenom {
            sender: env.contract.address.to_string(),
            subdenom: lp_symbol,
        }))
    } else {
        let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
        let code_id = manager_config.lp_token_type.get_cw20_code_id()?;
        let CodeInfoResponse { checksum, .. } = deps.querier.query_wasm_code_info(code_id)?;
        let seed = format!(
            "{}{}{}",
            asset_info,
            info.sender.into_string(),
            env.block.height
        );
        let salt = Binary::from(seed.as_bytes());

        let vault_lp_address = deps.api.addr_humanize(
            &instantiate2_address(&checksum, &creator, &salt)
                .map_err(|e| StdError::generic_err(e.to_string()))?,
        )?;

        let lp_asset = AssetInfo::Token {
            contract_addr: vault_lp_address.into_string(),
        };

        VAULTS.save(
            deps.storage,
            asset_info_reference,
            &Vault {
                asset_info,
                lp_asset: lp_asset.clone(),
                fees,
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        let lp_token_name = format!("{asset_label}-LP");

        Ok::<CosmosMsg, ContractError>(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
            admin: None,
            code_id,
            label: lp_token_name.to_owned(),
            msg: to_binary(&TokenInstantiateMsg {
                name: lp_token_name,
                symbol: LP_SYMBOL.to_string(),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.to_string(),
                    cap: None,
                }),
            })?,
            funds: vec![],
            salt,
        }))
    }?;

    Ok(Response::default()
        .add_message(message)
        .add_attribute("action", "create_vault".to_string())
        .add_attributes(attributes))
}

/// Updates the manager config
pub fn update_manager_config(
    deps: DepsMut,
    info: MessageInfo,
    fee_collector_addr: Option<String>,
    vault_creation_fee: Option<Asset>,
    cw20_lp_code_id: Option<u64>,
    flash_loan_enabled: Option<bool>,
    deposit_enabled: Option<bool>,
    withdraw_enabled: Option<bool>,
) -> Result<Response, ContractError> {
    validate_owner(deps.storage, OWNER, info.sender)?;

    let new_config = MANAGER_CONFIG.update::<_, ContractError>(deps.storage, |mut config| {
        if let Some(new_fee_collector_addr) = fee_collector_addr {
            config.fee_collector_addr = deps.api.addr_validate(&new_fee_collector_addr)?;
        }

        if let Some(vault_creation_fee) = vault_creation_fee {
            config.vault_creation_fee = vault_creation_fee;
        }

        if let Some(new_token_id) = cw20_lp_code_id {
            match config.lp_token_type {
                LpTokenType::Cw20(_) => {
                    config.lp_token_type = LpTokenType::Cw20(new_token_id);
                }
                LpTokenType::TokenFactory => {
                    return Err(ContractError::InvalidLpTokenType {});
                }
            }
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
        (
            "fee_collector_addr",
            &new_config.fee_collector_addr.into_string(),
        ),
        ("lp_token_type", &new_config.lp_token_type.to_string()),
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

/// Updates the fees for the vault of the given asset
pub fn update_vault_fees(
    deps: DepsMut,
    info: MessageInfo,
    vault_asset_info: AssetInfo,
    vault_fee: VaultFee,
) -> Result<Response, ContractError> {
    validate_owner(deps.storage, OWNER, info.sender)?;

    let mut vault = VAULTS
        .may_load(deps.storage, vault_asset_info.get_reference())?
        .ok_or(ContractError::NonExistentVault {})?;

    vault_fee.is_valid()?;
    vault.fees = vault_fee.clone();

    Ok(Response::default().add_attributes(vec![
        ("action", "update_vault_fees".to_string()),
        ("vault_asset_info", vault_asset_info.to_string()),
        ("vault_fee", vault_fee.to_string()),
    ]))
}

pub fn remove_vault(
    deps: DepsMut,
    info: MessageInfo,
    asset_info: AssetInfo,
) -> Result<Response, ContractError> {
    validate_owner(deps.storage, OWNER, info.sender)?;

    if let Ok(None) = VAULTS.may_load(deps.storage, asset_info.get_reference()) {
        return Err(ContractError::NonExistentVault {});
    }

    VAULTS.remove(deps.storage, asset_info.get_reference())?;

    Ok(Response::default().add_attributes(vec![
        ("method", "remove_vault".to_string()),
        ("asset_info", asset_info.to_string()),
    ]))
}

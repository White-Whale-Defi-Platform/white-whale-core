use cosmwasm_std::{
    attr, instantiate2_address, to_json_binary, Attribute, Binary, CodeInfoResponse, CosmosMsg,
    DepsMut, Env, MessageInfo, Response, StdError, Uint128, WasmMsg,
};
use cw20::MinterResponse;
use sha2::{Digest, Sha256};

use white_whale_std::constants::LP_SYMBOL;
use white_whale_std::pool_network::asset::{Asset, AssetInfo};
use white_whale_std::pool_network::token::InstantiateMsg as TokenInstantiateMsg;
#[cfg(any(
    feature = "token_factory",
    feature = "osmosis_token_factory",
    feature = "injective"
))]
use white_whale::tokenfactory;
use white_whale_std::vault_manager::{LpTokenType, Vault, VaultFee};

use crate::state::{get_vault_by_identifier, CONFIG, VAULTS, VAULT_COUNTER};
use crate::ContractError;
use white_whale_std::whale_lair::fill_rewards_msg;

/// Creates a new vault
pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo,
    fees: VaultFee,
    vault_identifier: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let denom = match config.vault_creation_fee.info.clone() {
        // this will never happen as the fee is always native, enforced when instantiating the contract
        AssetInfo::Token { .. } => "".to_string(),
        AssetInfo::NativeToken { denom } => denom,
    };

    // verify fee payment
    let amount = cw_utils::must_pay(&info, denom.as_str())?;
    if amount < config.vault_creation_fee.amount {
        return Err(ContractError::InvalidVaultCreationFee {
            amount,
            expected: config.vault_creation_fee.amount,
        });
    }

    let mut messages: Vec<CosmosMsg> = vec![];

    // send vault creation fee to whale lair
    let creation_fee = vec![Asset {
        info: config.vault_creation_fee.info,
        amount: config.vault_creation_fee.amount,
    }];

    // send protocol fee to whale lair
    messages.push(fill_rewards_msg(
        config.whale_lair_addr.into_string(),
        creation_fee,
    )?);

    let vault_id = VAULT_COUNTER.load(deps.storage)?;
    // if no identifier is provided, use the vault counter (id) as identifier
    let identifier = vault_identifier.unwrap_or(vault_id.to_string());

    // check if there is an existing vault with the given identifier
    let vault = get_vault_by_identifier(&deps.as_ref(), identifier.clone());
    if vault.is_ok() {
        return Err(ContractError::ExistingVault {
            asset_info,
            identifier,
        });
    }

    // check the vault fees are valid
    fees.is_valid()?;

    let asset_label = asset_info.clone().get_label(&deps.as_ref())?;
    let mut attributes = Vec::<Attribute>::new();
    attributes.push(attr("vault_identifier", identifier.clone()));

    let message = if config.lp_token_type == LpTokenType::TokenFactory {
        #[cfg(all(
            not(feature = "token_factory"),
            not(feature = "osmosis_token_factory"),
            not(feature = "injective")
        ))]
        return Err(ContractError::TokenFactoryNotEnabled {});

        let lp_symbol = format!("{asset_label}.vault.{identifier}.{LP_SYMBOL}");
        let denom = format!("{}/{}/{}", "factory", env.contract.address, lp_symbol);
        let lp_asset = AssetInfo::NativeToken { denom };

        VAULTS.save(
            deps.storage,
            identifier.clone(),
            &Vault {
                asset: Asset {
                    info: asset_info,
                    amount: Uint128::zero(),
                },
                lp_asset: lp_asset.clone(),
                fees,
                identifier,
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        #[cfg(any(
            feature = "token_factory",
            feature = "osmosis_token_factory",
            feature = "injective"
        ))]
        Ok(tokenfactory::create_denom::create_denom(
            env.contract.address,
            lp_symbol,
        ))
    } else {
        let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;
        let code_id = config.lp_token_type.get_cw20_code_id()?;
        let CodeInfoResponse { checksum, .. } = deps.querier.query_wasm_code_info(code_id)?;
        let seed = format!(
            "{}{}{}{}",
            asset_label,
            identifier,
            info.sender.into_string(),
            env.block.height
        );
        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let salt: Binary = hasher.finalize().to_vec().into();


        let vault_lp_address = deps.api.addr_humanize(
            &instantiate2_address(&checksum, &creator, &salt)
                .map_err(|e| StdError::generic_err(e.to_string()))?,
        )?;

        let lp_asset = AssetInfo::Token {
            contract_addr: vault_lp_address.into_string(),
        };

        VAULTS.save(
            deps.storage,
            identifier.clone(),
            &Vault {
                asset: Asset {
                    info: asset_info,
                    amount: Uint128::zero(),
                },
                lp_asset: lp_asset.clone(),
                fees,
                identifier,
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        let lp_token_name = format!("{asset_label}-LP");

        Ok::<CosmosMsg, ContractError>(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
            admin: None,
            code_id,
            label: lp_token_name.to_owned(),
            msg: to_json_binary(&TokenInstantiateMsg {
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
    vault_creation_fee: Option<Asset>,
    cw20_lp_code_id: Option<u64>,
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
        ("whale_lair_addr", &new_config.whale_lair_addr.into_string()),
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

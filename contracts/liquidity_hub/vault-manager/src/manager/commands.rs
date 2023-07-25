use cosmwasm_std::{
    attr, instantiate2_address, to_binary, Attribute, Binary, CodeInfoResponse, CosmosMsg, DepsMut,
    Env, MessageInfo, Response, StdError, WasmMsg,
};
use cw20::MinterResponse;

use white_whale::constants::LP_SYMBOL;
use white_whale::pool_network::asset::AssetInfo;
use white_whale::pool_network::token::InstantiateMsg as TokenInstantiateMsg;
use white_whale::traits::AssetReference;
use white_whale::vault_manager::{LpTokenType, Vault, VaultFee};

use crate::state::{MANAGER_CONFIG, VAULTS};
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
    let amount = cw_utils::must_pay(&info, denom.as_str()).unwrap();
    if amount != manager_config.vault_creation_fee.amount {
        return Err(ContractError::InvalidVaultCreationFee {
            amount,
            expected: manager_config.vault_creation_fee.amount,
        });
    }

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
        let lp_symbol = format!("{asset_label}.{LP_SYMBOL}");
        let denom = format!("{}/{}/{}", "factory", env.contract.address, lp_symbol);
        let lp_asset = AssetInfo::NativeToken { denom };

        VAULTS.save(
            deps.storage,
            asset_info_reference,
            &Vault {
                asset_info,
                asset_info_reference: asset_info_reference.to_vec(),
                lp_asset: lp_asset.clone(),
                fees,
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        #[cfg(any(feature = "token_factory", feature = "osmosis_token_factory"))]
        return Ok(<MsgCreateDenom as Into<CosmosMsg>>::into(MsgCreateDenom {
            sender: env.contract.address.to_string(),
            subdenom: lp_symbol,
        }));
        #[allow(unreachable_code)]
        Err(ContractError::TokenFactoryNotEnabled {})
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
                asset_info_reference: asset_info_reference.to_vec(),
                lp_asset: lp_asset.clone(),
                fees,
            },
        )?;

        attributes.push(attr("lp_asset", lp_asset.to_string()));

        let lp_token_name = format!("{asset_label}-LP");

        Ok(CosmosMsg::Wasm(WasmMsg::Instantiate2 {
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
        .add_attributes(vec![("method", "create_vault")]))
}

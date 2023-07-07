use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

use white_whale::fee::VaultFee;
use white_whale::pool_network::asset::AssetInfo;
use white_whale::traits::AssetReference;
use white_whale::vault_manager::Vault;

use crate::ContractError;
use crate::state::{MANAGER_CONFIG, VAULTS};

pub fn create_vault(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset_info: AssetInfo,
    fees: VaultFee,
    token_factory_lp: bool
) -> Result<Response, ContractError> {
    let manager_config = MANAGER_CONFIG.load(deps.storage)?;

    let denom = match manager_config.vault_creation_fee.info {
        AssetInfo::Token { .. } => "".to_string(),
        AssetInfo::NativeToken { denom} => denom,
    };

    // verify fee payment
    let amount = cw_utils::must_pay(&info, denom.as_str()).unwrap();
    if amount != manager_config.vault_creation_fee.amount {
        return Err(ContractError::InvalidVaultCreationFee { amount, expected: manager_config.vault_creation_fee.amount });
    }

    let binding = asset_info.clone();
    let asset_info_reference = binding.get_reference();

    // check that existing vault does not exist
    let vault = VAULTS.may_load(deps.storage, asset_info_reference)?;
    if let Some(_) = vault {
        return Err(ContractError::ExistingVault { asset_info });
    }

    // check the fees are valid
    fees.flash_loan_fee.is_valid()?;
    fees.protocol_fee.is_valid()?;

    //todo abstract this kind of behavior into the ww package
    if token_factory_lp {

    } else {

    }


    //todo use instantiate2 for predicting the address of the cw20 token if
    // create the vault
    VAULTS.save(
        deps.storage,
        asset_info_reference,
        &Vault{
            asset_info,
            asset_info_reference : asset_info_reference.to_vec(),
            lp_asset: AssetInfo::NativeToken { denom: "factory/something/uLP".to_string() },
            fees,
        },
    )?;

    Ok(Response::default()
        .add_attributes(vec![("method", "create_vault")]))
}

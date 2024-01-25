use cosmwasm_std::testing::mock_info;
use cosmwasm_std::{Addr, MessageInfo};
use cw_multi_test::{App, Executor};

use white_whale_std::fee::Fee;
use white_whale_std::pool_network::asset::{AssetInfo, PairType};
use white_whale_std::pool_network::pair::PoolFee;

use crate::tests::store_code::{store_cw20_token_code, store_pool};

pub fn mock_creator() -> MessageInfo {
    mock_info("creatorcreator", &[])
}

/// Instantiates a pool
pub fn app_mock_instantiate(
    app: &mut App,
    asset_infos: [AssetInfo; 2],
    asset_decimals: [u8; 2],
) -> Addr {
    let pool_id = store_pool(app);
    let token_id = store_cw20_token_code(app);

    let creator = mock_creator().sender;

    app.instantiate_contract(
        pool_id,
        creator.clone(),
        &white_whale_std::pool_network::pair::InstantiateMsg {
            asset_infos,
            token_code_id: token_id,
            asset_decimals,
            pool_fees: PoolFee {
                protocol_fee: Fee {
                    share: Default::default(),
                },
                swap_fee: Fee {
                    share: Default::default(),
                },
                burn_fee: Fee {
                    share: Default::default(),
                },
            },
            fee_collector_addr: creator.to_string(),
            pair_type: PairType::ConstantProduct,
            token_factory_lp: false,
        },
        &[],
        "mock pool",
        None,
    )
    .unwrap()
}

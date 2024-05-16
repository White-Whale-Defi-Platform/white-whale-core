use crate::helpers::extract_pool_identifier;
use cosmwasm_std::coin;

#[test]
fn test_extract_pool_identifier() {
    let denom_1 = "non_whitelisted_asset";
    let denom_2 = "factory/contract100/uluna-uwhale.pool.random_identifier.uLP";
    let denom_3 = "factory/contract100/uluna-uwhale.pool..pool./.pool.crazy.pool.identifier.uLP";
    let denom_4 = "factory/contract100/uluna-uwhale.pool.messy_.pool._identifier.uLP";
    let denom_5 = "factory/contract100/uluna-uwhale.pool./hacky_.pool./_identifier.uLP";
    let denom_6 = "factory/contract100/uluna-uwhale.pair.1.uLP";
    let denom_7 = "factory/contract100/uluna-uwhale.pair.1";
    let denom_8 = "factory/contract100/bWHALE";

    let res = extract_pool_identifier(denom_1);
    assert!(res.is_none());

    let res = extract_pool_identifier(denom_2);
    assert_eq!(res.unwrap(), "random_identifier");
    let res = extract_pool_identifier(denom_3);
    assert_eq!(res.unwrap(), ".pool./.pool.crazy.pool.identifier");

    let res = extract_pool_identifier(denom_4);
    assert_eq!(res.unwrap(), "messy_.pool._identifier");

    let res = extract_pool_identifier(denom_5);
    assert_eq!(res.unwrap(), "/hacky_.pool./_identifier");

    let res = extract_pool_identifier(denom_6);
    assert!(res.is_none());

    let res = extract_pool_identifier(denom_7);
    assert!(res.is_none());

    let res = extract_pool_identifier(denom_8);
    assert!(res.is_none());
}

use crate::helpers::extract_pool_identifier;
use test_case::test_case;

#[test_case("non_whitelisted_asset", None)]
#[test_case(
    "factory/contract1/uluna-uwhale.pool.random_identifier.uLP",
    Some("random_identifier")
)]
#[test_case(
    "factory/contract2/uluna-uwhale.pool..pool./.pool.crazy.pool.identifier.uLP",
    Some(".pool./.pool.crazy.pool.identifier")
)]
#[test_case(
    "factory/contract3/uluna-uwhale.pool.messy_.pool._identifier.uLP",
    Some("messy_.pool._identifier")
)]
#[test_case(
    "factory/contract4/uluna-uwhale.pool./hacky_.pool./_identifier.uLP",
    Some("/hacky_.pool./_identifier")
)]
#[test_case("factory/contract5/uluna-uwhale.pair.1.uLP", None)]
#[test_case("factory/contract6/uluna-uwhale.pair.1", None)]
#[test_case("factory/contract7/bWHALE", None)]
fn test_extract_pool_identifier(denom: &str, expected: Option<&str>) {
    let res = extract_pool_identifier(denom);

    if res.is_none() {
        assert!(expected.is_none());
    } else {
        assert_eq!(res.unwrap(), expected.unwrap());
    }
}

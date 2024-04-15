#[test]
fn test_calculate_weight() {
    use crate::position::helpers::calculate_weight;
    use cosmwasm_std::coin;
    use cosmwasm_std::Uint128;

    let weight = calculate_weight(&coin(100, "uwhale"), 86400u64).unwrap();
    assert_eq!(weight, Uint128::new(100));

    // 1 month
    let weight = calculate_weight(&coin(100, "uwhale"), 2629746).unwrap();
    assert_eq!(weight, Uint128::new(117));

    // 3 months
    let weight = calculate_weight(&coin(100, "uwhale"), 7889238).unwrap();
    assert_eq!(weight, Uint128::new(212));

    // 6 months
    let weight = calculate_weight(&coin(100, "uwhale"), 15778476).unwrap();
    assert_eq!(weight, Uint128::new(500));

    // 1 year
    let weight = calculate_weight(&coin(100, "uwhale"), 31556926).unwrap();
    assert_eq!(weight, Uint128::new(1599));
}

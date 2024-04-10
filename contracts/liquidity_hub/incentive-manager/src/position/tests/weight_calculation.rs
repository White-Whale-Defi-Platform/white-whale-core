#[test]
fn test_calculate_weight() {
    use crate::position::helpers::calculate_weight;
    use cosmwasm_std::coin;

    let weight = calculate_weight(&coin(100, "uwhale"), 86400u64).unwrap();
    println!("1 day Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 1209600).unwrap();
    println!("2 weeks Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 2629746).unwrap();
    println!("1 month Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 5259492).unwrap();
    println!("2 months Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 7889238).unwrap();
    println!("3 months Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 10518984).unwrap();
    println!("4 months Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 13148730).unwrap();
    println!("5 months Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 15778476).unwrap();
    println!("6 months Weight: {:?}", weight);

    let weight = calculate_weight(&coin(100, "uwhale"), 31556926).unwrap();
    println!("1 year Weight: {:?}", weight);
}

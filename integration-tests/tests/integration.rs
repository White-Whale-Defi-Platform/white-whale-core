use cosmwasm_std::coin;

use white_whale_std::pool_manager::PoolType;

use crate::common::helpers;
use crate::common::suite::TestingSuite;

mod common;

#[test]
fn epic_test() {
    let mut suite = TestingSuite::default_with_balances(vec![
        coin(1_000_000_000_000u128, "uwhale"),
        coin(1_000_000_000_000u128, "uosmo"),
        coin(1_000_000_000_000u128, "uusdc"),
        coin(1_000_000_000_000u128, "uusdt"),
        // ibc token is stablecoin
        coin(
            1_000_000_000_000u128,
            "ibc/BEFB9AB13AB43157A0AF6254AD4B1F565AC0CA0C1760B8339BE7B9E2996F7752",
        ),
        coin(
            1_000_000_000_000u128,
            "factory/migaloo193lk767456jhkzddnz7kf5jvuzfn67gyfvhc40/ampWHALE",
        ),
        coin(
            1_000_000_000_000u128,
            "factory/migaloo1ludaslnu24p5eftw499f7ngsc2jkzqdsrvxt75/bWHALE",
        ),
        coin(
            1_000_000_000_000u128,
            "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5",
        ),
        coin(1_000_000_000_000_000u128, "btc"),
        coin(1_000_000_000_000_000_000_000_000u128, "inj"),
    ]);

    suite.instantiate();

    let [alice, bob, carol, dave, sybil] = [
        suite.senders[0].clone(),
        suite.senders[1].clone(),
        suite.senders[2].clone(),
        suite.senders[3].clone(),
        suite.senders[4].clone(),
    ];

    // create some pools
    helpers::pools::create_pools(&mut suite, alice.clone());
    helpers::vaults::create_vaults(&mut suite, bob.clone());
}

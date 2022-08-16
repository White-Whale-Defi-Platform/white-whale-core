# Pool Network

White Whale's pool network is based on [Terraswap](https://github.com/terraswap/terraswap), a Uniswap-inspired automated market-maker (AMM) protocol.

## Contracts

| Name                                               | Description                                  |
| -------------------------------------------------- | -------------------------------------------- |
| [`terraswap_factory`](terraswap_factory) |                                              |
| [`terraswap_pair`](terraswap_pair)       |                                              |
| [`terraswap_router`](terraswap_router)   |                                              |
| [`terraswap_token`](terraswap_token)     | CW20 (ERC20 equivalent) token implementation |

## Running the Pool Network

This repo is a submodule of [Migaloo Core](https://github.com/White-Whale-Defi-Platform/migaloo-core), White Whale's core infrastructure repository.

You will need Rust 1.44.1+ with wasm32-unknown-unknown target installed.

You can run unit tests on each contract directory via:

```
cargo test
```

To compile each individual contract, you can run:

```
cargo build
cargo wasm
```

Or for a production-ready (optimized) build, run the following from the `migaloo-core` repository:

```
scripts/build_release.sh
```

The optimized contracts are generated in the artifacts/ directory.

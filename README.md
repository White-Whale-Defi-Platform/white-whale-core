<a href="https://whitewhale.money/">
  <h1 align="center">
    <picture>
      <img alt="Flutter" src="https://miro.medium.com/max/1400/1*29OYRJqqddosWtWo-c3TYQ.png">
    </picture>
  </h1>
</a>

[![codecov](https://codecov.io/github/White-Whale-Defi-Platform/migaloo-core/branch/main/graph/badge.svg?token=Y8S6P1KBS2)](https://codecov.io/github/White-Whale-Defi-Platform/migaloo-core)
[![CII Best Practices](https://bestpractices.coreinfrastructure.org/projects/6401/badge)](https://bestpractices.coreinfrastructure.org/projects/6401)
[![Discord badge][]][Discord invite]
[![Twitter handle][]][Twitter badge]
[![first-timers-only](https://img.shields.io/badge/first--timers--only-friendly-blue.svg?style=flat-square)](https://www.firsttimersonly.com/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](https://makeapullrequest.com)


[Discord invite]: https://discord.com/invite/tSxyyCWgYX
[Discord badge]: https://img.shields.io/discord/908044702794801233
[Twitter handle]: https://img.shields.io/twitter/follow/WhiteWhaleDefi.svg?style=social&label=Follow
[Twitter badge]: https://twitter.com/intent/follow?screen_name=WhiteWhaleDefi

## Getting started

To get started with `migaloo-core`, we encourage you to go through our [contributing guide](./CONTRIBUTING.md) to see the 
different ways to contribute to the project.

## Resources

1. [Website](https://whitewhale.money/)
2. [LitePaper](https://whitewhale.money/LitepaperV2.pdf)
3. [Docs](https://ww0-1.gitbook.io/migaloo-docs/)
4. [Discord](https://discord.com/invite/tSxyyCWgYX)
5. [Twitter](https://twitter.com/WhiteWhaleDefi)
6. [Telegram](https://t.me/whitewhaleofficial)

## Building and Deploying Migaloo

To build and deploy Migaloo´s smart contracts we have created a series of scripts under `scripts/`. You need at least Rust v1.64.0 to compile the contracts. 

### Build scripts

- `build_release.sh`: builds the project artifacts, optimized for production.
- `build_schemas.sh`: generates schemas for the contracts.
- `check_artifacts_size.sh`: validates the size of the optimized artifacts. The default maximum size is 600 kB, though 
it is customizable by passing the number of kB to the script. For example `check_artifacts_size.sh 400` verifies the 
artifacts are under 400 kB.

### Deployment scripts

The deployment scripts are found under `scripts/deployment/`. The following is the structure found on under this folder:

```bash
.
├── deploy_env
│   ├── base.env
│   ├── chain_env.sh
│   ├── mainnets
│   │   ├── chihuahua.env
│   │   ├── juno.env
│   │   └── terra.env
│   ├── mnemonics
│   │   ├── deployer_mnemonic_testnet.txt
│   │   └── deployer_mnemonic.txt
│   └── testnets
│       ├── archway.env
│       ├── injective.env
│       ├── juno.env
│       ├── local.env
│       └── terra.env
├── deploy_liquidity_hub.sh
├── deploy_pool.sh
├── deploy_vault.sh
├── input
│   ├── pool.json
│   └── vault.json
├── output
│   ├── uni-5_liquidity_hub_contracts.json
│   ├── uni-5_pools.json
│   └── uni-5_vaults.json
└── wallet_importer.sh
```

There are three main scripts: `deploy_liquidity_hub.sh`, `deploy_pool.sh` and `deploy_vault.sh`. The rest of the scripts 
in there are used as auxiliary scripts by the main three listed before.

The `deploy_env/` folder contains env files defining the parameters for the blockchain where the deployment is going to occur, 
whether it is a mainnet or testnet deployment.  

The `input/` folder is used for adding json files containing the config parameters when deploying pools or vaults.
The `output/` folder is where the scripts will write the data regarding the deployment, in json format. The name of the file
follows the following nomenclature: `"chain_id"_liquidity_hub_contracts`, `"chain_id"_pools`, `"chain_id"_vaults`.

- `deploy_liquidity_hub.sh`: deploys the liquidity hubs. It can deploy the entire LH or parts of it. To learn how to use it, 
run the script with the `-h` flag.
- `deploy_pool.sh`: deploys a pool based on the configuration specified at `input/pool.json`. To learn how to use it, 
run the script with the `-h` flag.
- `deploy_vault.sh`: deploys a vault based on the configuration specified at `input/vault.json`. To learn how to use it, 
run the script with the `-h` flag.

Notice that to deploy a pool or vault you need to have deployed the pool or vault factory respectively.

Here are some examples:

```bash
scripts/deployment/deploy_liquidity_hub.sh -c juno -d all
scripts/deployment/deploy_liquidity_hub.sh -c juno -d vault-network
scripts/deployment/deploy_pool-sh -c juno -p scripts/deployment/input/pool.json
scripts/deployment/deploy_vault-sh -c juno -v scripts/deployment/input/vault.json
```

## Testing

To run the tests, run `cargo test`. You can also run `cargo tarpaulin -v` to get test code coverage.

## Disclaimer

**Use the contracts and the White Whale app at your own risk!**

## Audit

Migaloo core contracts have been audited by [SCV-Security](https://www.scv.services/). The report can be found [here](https://github.com/SCV-Security/PublicReports/blob/main/CW/WhiteWhale/White%20Whale%20-%20Migaloo%20Audit%20Report%20v1.0.pdf).

## Contributing

[Contributing Guide](./docs/CONTRIBUTING.md)

[Code of Conduct](./docs/CODE_OF_CONDUCT.md)

[Security Policies and Procedures](./docs/SECURITY.md)

[License](./LICENSE)

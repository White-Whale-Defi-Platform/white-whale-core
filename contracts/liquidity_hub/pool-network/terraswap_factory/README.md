# TerraSwap Factory

Migaloo's factory contract is used to create pair (pool) contracts. Pools are comprised of two tokens, which can be either
native, ibc or cw20 tokens. Once a pool is created it's stored in state, meaning the factory acts as a pool registry,
which can be queried for reference. Note that the pool factory is permissioned, meaning the messages can only be executed
by the owner of the contract.

To find out more about the factory contract, refer to the [Migaloo docs](https://ww0-1.gitbook.io/migaloo-docs/liquidity-hub/overview-1/terraswap-factory).

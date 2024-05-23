# Pool Manager

The Pool Manager is a contract that handles pools in the Migaloo DEX.

Pools can contain two or three assets and are used to provide liquidity to the DEX. The Pool Manager is responsible for
creating pools and handling swaps, whether they are single or multi-hop operations.

## How it works

The following is a high-level overview of how the Pool Manager works. It touches on some technical details, assisting
developers in understanding the contract's inner workings while also providing a general understanding of the contract's
functionality, so a regular user can understand how to interact with it.

### Pool Creation

Creating pools is a simple and permissionless process. A user can call the `CreatePool` message, with the desired pool
parameters such as asset denoms, fees, and pool type among others, together with the pool creation fee. The pool creation
fee is a protocol fee that is sent to the Bonding Manager via the `FillRewards` message. There can be multiple pools
for the same asset pair, though each pool must have a unique identifier. Pools cannot be removed or updated once
created, so it is important to get the parameters right from the start.

The liquidity in a given pool is tracked with LP tokens, which are minted via the Token Factory module by the Pool Manager.
These tokens represent the user's share of a pool's liquidity, and they can be used to redeem the assets in the pool.

Pool information is stored in the `POOLS` map, containing information such as the asset denoms and decimals, the LP denom,
the assets in the pool (balance), the pool type and pool fees.

A pool can be of two types: `ConstantProduct` (xyk) or `StableSwap`. The `ConstantProduct` type is suitable for assets that
may have varying values and are not intended to be equivalent. The `StableSwap` type is suitable for assets that are
meant to be the same and whose values should be approximately the same, such as stablecoins.

### Deposits and Withdrawals

Users can deposit and withdraw assets from the pools at any time. To deposit, users must call the `ProvideLiquidity`
message, together with the pool identifier and the assets to deposit among other parameters. For pools with two assets,
it is possible to provide liquidity with a single asset. The Pool Manager will swap half of the provided asset for the
other asset in the pool, ensuring the pool's balance is kept in check.

Once the user has provided liquidity, they will receive LP tokens in return proportional to the amount of liquidity
provided.

To withdraw liquidity, users must call the `WithdrawLiquidity` message, with the pool identifier together with the LP
token to redeem the assets. The Pool Manager will burn the LP tokens and send the corresponding assets to the user,
updating the pool's balance accordingly.

### Swaps

Swaps are the main feature of the Pool Manager. Users can swap assets from one pool to another by using the `Swap` message.
If the swap is a single-hop operation, the Pool Manager will perform the swap directly. If the swap is a multi-hop operation,
the `ExecuteSwapOperations` message should be used instead, providing the route to follow for the swap to be executed
successfully.

After a swap takes place, the pool's balances are updated, and the fees are collected. The Bonding Manager receives the
protocol fee via the `FillRewards` message, while the swap fee remains in the pool to benefit the LP token holders,
increasing the pool's liquidity and thus the LP token value.

On Osmosis, there's an additional fee that is sent to the Osmosis community pool.

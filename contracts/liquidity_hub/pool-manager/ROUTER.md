# Pool Manager router

Previously in the pool-network repo a router contract is used to perform multihop swaps which involve more than 1 hop or route to complete a swap.
This functionality is needed in the pool manager but the implementation needs to be different.
In the pool manager there are many pairs and its possible to have multiple pairs with the same assets. For this reason we can't deterministically evaluate a pair key from just asstes. Instead a pool_idenitifer is used to identify a pair and each pair has one. This is what is used to specify a pair in the pool manager.

A multihop swap with this pair manager contract could look like the below

```
routes:
    - pool_id: '960'
      token_out_denom: uosmo
    - pool_id: '2'
      token_out_denom: uion

```

The above route would swap from a given route to uosmo and then from uosmo to uion.
Becuase the route is specified by pool_id we can have multiple routes with the same assets but when providing a pool ID and the asset we want we can determine the asset its trading against.

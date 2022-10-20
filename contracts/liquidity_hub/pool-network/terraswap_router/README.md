# Terraswap Router

The router contract is used to execute multi-hop swaps. Say there are two pools: ATOM-JUNO and JUNO-LUNA. There is no way
to directly swap ATOM for LUNA, as there is no ATOM-LUNA pool. With the router contract, it is possible to can concatenate
swap operations so that it becomes possible to swap ATOM for LUNA via JUNO, i.e. ATOM->JUNO->LUNA.

The router is mainly used by bots and the UI.

To find out more about the factory contract, refer to the [Migaloo docs](https://ww0-1.gitbook.io/migaloo-docs/liquidity-hub/overview-1/terraswap-factory).

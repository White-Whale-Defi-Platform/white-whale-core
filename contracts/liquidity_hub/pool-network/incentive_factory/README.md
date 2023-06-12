# Incentives

Migaloo's incentive factory contract is used to create incentive flows associated with LP tokens. Incentive contracts allow permissioned users to create an incentive contract associated with an LP token. Once an incentive contract is created for a LP token, it is stored in state, allowing the incentive factory to act as a incentive registry, which can be queried for reference. Note that the incentive factory is permissioned, meaning the messages can only be executed by the owner of the contract.

To find out more about the incentives contracts, refer to the [Migaloo docs](https://ww0-1.gitbook.io/migaloo-docs/liquidity-hub/overview-1/).

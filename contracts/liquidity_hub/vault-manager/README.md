# Vault Manager

The Vault Manager is the V2 iteration of the original White Whale vault network. This is a monolithic contract that 
handles all the vaults and flashloans on the Migaloo DEX.

Vaults are single collateral pools used primarily by bots to take arbitrage opportunities across different AMMs via 
flashloans. Flashloans have fees that are distributed among the users that have deposited into the vault where the 
The Vault Network is a WW-controlled collection of single asset vaults that bots use to access flashloan capital for arbitrage, liquidation, and other DeFi uses. By using the Flash Loan Vaults, arbitrage and liquidations happen locally in a capital-efficient manner and without capital requirements. That is, each arbitrageur or liquidator will no longer need their own capital on each local chain waiting idly to arb or to liquidate because they can access a flash loan for their capital needs. When an arbitrage opportunity arises, an arbitrageur takes a flash loan, arbs the local dex price versus the WW pool, and then pays back the loan plus the flash loan fee. The arbitrageur then keeps the profit without having used any of their own capital.

Depositors of tokens into flash loan vaults benefit from fees paid when their vault is accessed for flash loans; the greater the volume, the more fees generated. Flash loan vaults are a great source of yield with no impermanent loss.```

Maybe just use what we already had for the Vault network in docs ? First paragraph tells us all the differences we need to know so can just reuse this blurb I think 

## How it works

The following is a high-level overview of how the Vault Manager works. It touches on some technical details, assisting
developers in understanding the contract's inner workings, while also providing a general understanding of the contract's
functionality, so a regular user can understand how to interact with it.

### Vault Creation

Creating vaults is a simple and permissionless process. A user can call the `CreateVault` message, with the desired vault 
parameters together with the vault creation fee. The vault creation fee is a protocol fee that is sent to the Bonding 
Manager. There can be multiple vaults for the same asset, but each vault must have a unique identifier. Vaults cannot be
removed or updated once created, so it is important to get the parameters right from the start.

### Deposits and Withdrawals

Users can deposit and withdraw assets from the vaults at any time. The assets are stored in the vault's balance, and they  
are used to provide liquidity to the arbitrage bots.

### Flashloans

Flashloans are a powerful tool that allows bots and users to borrow assets from the vault without any collateral, with 
the condition that the assets plus the flashloan fees are returned within the same transaction, otherwise the transaction 
is reverted as if nothing had happened. 

A flashloan can be taken by calling the `FlashLoan` message, with the desired amount, vault identifier and the payload. 
The payload is a list of messages that will be executed in the same transaction, and it doesn't need to include the "payback" 
transaction as it is handled by the contract.

When a flashloan is taken, a boolean in the state is set to true on `ONGOING_FLASHLOAN`, so the funds can't be used to 
be deposited back into a vault or to take another flashloan. After the payload is executed, the `CallbackMsg::AfterFlashloan` 
is called. This makes sure the funds are back in the vault plus the fees. The profit made from the payload operations is 
sent back to the originator of the flashloan. The Bonding Manager receives the protocol fees via the `FillRewards` message 
and the users that have deposited into the vault where the flashloan was taken from receive the flashloan fees.
